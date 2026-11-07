//! The network axis's preload half (spec 08 §2): observe resolver answers,
//! judge outbound peers against the bound policy's `net:` grants.
//!
//! The policy judges PEER SPELLINGS — a grant names a DNS host or an IPv4
//! literal, but a `connect(2)` hands the shim a numeric `sockaddr`. The
//! bridge is the resolver-observation table: the interposed `getaddrinfo`
//! records which names resolved to which addresses (DNS itself is never
//! gated — the §2 carve-out: a query leaks a name, never payload bytes,
//! and a grant names hosts whose addresses must resolve), and the
//! interposed `connect`/`sendto`/`sendmsg` judge the peer as the candidate
//! set [numeric address, observed names…] against the policy.
//!
//! Judgment lives in the ENGINE (`tfs::context::net_check` — one policy
//! owner, one journal, one trace channel); this module owns only the
//! observation table and the candidate-set assembly. A peer whose name
//! never rode the resolver (a literal-IP connect) is judged by its numeric
//! spelling alone — grantable by an IPv4-literal grant.
//!
//! Table discipline: bounded (8k entries — a long-lived runtime that
//! resolves the internet stops LEARNING new names past the cap, it never
//! fails; a name it already learned keeps judging) and never blocking
//! beyond the mutex. Allocation happens here, post-constructor, exactly
//! where the real `getaddrinfo` already allocated.

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Mutex;

/// The observation table's capacity. Past it, new answers are not learned
/// (deny-by-default then judges a never-seen name by its numeric spelling
/// alone) — a bounded table beats an unbounded leak in a long-lived
/// process; the connect path's verdict stays correct for everything
/// already learned.
const OBSERVED_CAP: usize = 8192;

/// peer address → the names whose resolver answers carried it. `None` until
/// the first observation (the common no-network payload never allocates).
static OBSERVED: Mutex<Option<HashMap<IpAddr, Vec<String>>>> = Mutex::new(None);

/// Record one `getaddrinfo` answer: `name` resolved to `addrs`. Numeric
/// node spellings map to themselves (an identity entry is harmless — the
/// numeric spelling is judged first anyway).
pub fn observe(name: &str, addrs: impl Iterator<Item = IpAddr>) {
    let name = name.to_lowercase();
    let mut guard = OBSERVED.lock().unwrap();
    let table = guard.get_or_insert_with(HashMap::new);
    for addr in addrs {
        if table.len() >= OBSERVED_CAP && !table.contains_key(&addr) {
            continue;
        }
        let names = table.entry(addr).or_default();
        if !names.contains(&name) {
            names.push(name.clone());
        }
    }
}

/// Judge one outbound peer: the numeric address first (literal-IP grants),
/// then every observed name for it (name grants), all at the connection's
/// port. Ok(()) passes; Err(EPERM) is the jail's refusal (journaled by the
/// engine, once, against the numeric spelling).
pub fn judge(ip: IpAddr, port: u16) -> Result<(), i32> {
    let mut spellings: Vec<String> = Vec::with_capacity(2);
    spellings.push(ip.to_string());
    if let Some(names) = OBSERVED.lock().unwrap().as_ref().and_then(|t| t.get(&ip)) {
        spellings.extend(names.iter().cloned());
    }
    let refs: Vec<&str> = spellings.iter().map(String::as_str).collect();
    tfs::context::context()
        .read()
        .unwrap()
        .net_check(&refs, port)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn judge_str(ip: &str, port: u16) -> Result<(), i32> {
        judge(ip.parse().unwrap(), port)
    }

    /// One policy per test (the crate-wide TEST_LOCK serializes): deny
    /// with the given net grants, no journal (the verdict is the test's
    /// subject, not the audit line).
    fn bind_policy(net: &[&str]) {
        let grants: Vec<tfs::policy::NetGrant> = net
            .iter()
            .map(|g| tfs::policy::NetGrant::parse(g).unwrap())
            .collect();
        let policy =
            tfs::policy::HostPolicy::bind(tfs::policy::PolicyDefault::Deny, vec![], vec![], grants)
                .unwrap();
        tfs::context::context()
            .write()
            .unwrap()
            .set_host_policy(policy, None);
    }

    fn reset_policy() {
        tfs::context::context()
            .write()
            .unwrap()
            .set_host_policy(tfs::policy::HostPolicy::open(), None);
    }

    #[test]
    fn deny_judges_by_numeric_and_observed_name() {
        let _g = crate::TEST_LOCK.lock().unwrap();
        bind_policy(&["example.com:443", "203.0.113.9"]);
        // A name grant does not judge a never-resolved address…
        assert_eq!(judge_str("93.184.216.34", 443), Err(libc::EPERM));
        // …until the resolver observation maps it.
        observe(
            "Example.COM",
            ["93.184.216.34".parse::<IpAddr>().unwrap()].into_iter(),
        );
        assert_eq!(judge_str("93.184.216.34", 443), Ok(()));
        // The port pin bites: 80 is not the granted port.
        assert_eq!(judge_str("93.184.216.34", 80), Err(libc::EPERM));
        // The literal-IP grant needs no observation, any port.
        assert_eq!(judge_str("203.0.113.9", 8443), Ok(()));
        // A peer nobody granted stays refused.
        assert_eq!(judge_str("198.51.100.7", 443), Err(libc::EPERM));
        reset_policy();
    }

    #[test]
    fn open_default_passes_everything() {
        let _g = crate::TEST_LOCK.lock().unwrap();
        reset_policy();
        assert_eq!(judge_str("198.51.100.99", 22), Ok(()));
    }

    #[test]
    fn ipv6_peers_judge_by_observed_name() {
        let _g = crate::TEST_LOCK.lock().unwrap();
        bind_policy(&["v6.example.com"]);
        let v6 = IpAddr::V6(std::net::Ipv6Addr::LOCALHOST);
        assert_eq!(judge(v6, 443), Err(libc::EPERM));
        observe("v6.example.com", [v6].into_iter());
        assert_eq!(judge(v6, 443), Ok(()));
        reset_policy();
    }
}
