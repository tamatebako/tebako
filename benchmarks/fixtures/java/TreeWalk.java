import java.nio.file.*;
import java.util.stream.Stream;
public class TreeWalk {
    public static void main(String[] a) throws Exception {
        int passes = a.length > 0 ? Integer.parseInt(a[0]) : 1;
        Path home = Paths.get(System.getProperty("java.home"));
        long n = 0;
        for (int p = 0; p < passes; p++) {
            try (Stream<Path> s = Files.walk(home)) {
                n += s.filter(Files::isRegularFile).mapToLong(f -> {
                    try { return Files.size(f); } catch (Exception e) { return 0; }
                }).count();
            }
        }
        System.out.println(home + " " + n);
    }
}
