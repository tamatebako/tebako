import java.nio.file.*;
public class TreeWalk {
    public static void main(String[] a) throws Exception {
        Path home = Paths.get(System.getProperty("java.home"));
        long n = Files.walk(home).filter(Files::isRegularFile).mapToLong(p -> {
            try { return Files.size(p); } catch (Exception e) { return 0; }
        }).count();
        System.out.println(home + " " + n);
    }
}
