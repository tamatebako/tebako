import java.util.ArrayList;
import java.util.HashMap;
import java.util.HashSet;
import java.util.TreeMap;
import java.util.TreeSet;

// stdlib-load equivalent for the JVM: exercise the core collections
// framework (the class-load + JIT warm question), then exit.
public class Collections {
    public static void main(String[] a) {
        ArrayList<Integer> list = new ArrayList<>();
        for (int i = 0; i < 100000; i++) list.add(i);
        java.util.Collections.shuffle(list, new java.util.Random(42));
        java.util.Collections.sort(list);
        HashSet<Integer> set = new HashSet<>(list);
        TreeSet<Integer> tree = new TreeSet<>(list);
        HashMap<Integer, Integer> map = new HashMap<>();
        TreeMap<Integer, Integer> sorted = new TreeMap<>();
        for (int i : list) { map.put(i, i * 2); sorted.put(i, i * 3); }
        System.out.println(list.size() + set.size() + tree.size() + map.size() + sorted.size());
    }
}
