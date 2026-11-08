import java.io.InputStream;
import java.nio.file.Files;
import java.nio.file.Paths;

// Sequential read bandwidth: read the fixture file (argv[0]) in 1 MiB
// chunks, argv[1] passes (default 1). The tebako arm reads it from inside
// the mounted image; the on-system arm reads the same bytes from the host
// path. The passes amortize the fixed boot cost so the ratio measures the
// read path, not the launch (spec 27 §10).
public class IoRead {
    public static void main(String[] a) throws Exception {
        int passes = a.length > 1 ? Integer.parseInt(a[1]) : 1;
        byte[] buf = new byte[1048576];
        long total = 0;
        for (int p = 0; p < passes; p++) {
            try (InputStream in = Files.newInputStream(Paths.get(a[0]))) {
                int n;
                while ((n = in.read(buf)) != -1) total += n;
            }
        }
        System.out.println(total);
    }
}
