import java.io.InputStream;
import java.nio.file.Files;
import java.nio.file.Paths;

// Sequential read bandwidth: read the fixture file (argv[0]) in 1 MiB
// chunks. The tebako arm reads it from inside the mounted image; the
// on-system arm reads the same bytes from the host path.
public class IoRead {
    public static void main(String[] a) throws Exception {
        byte[] buf = new byte[1048576];
        long total = 0;
        try (InputStream in = Files.newInputStream(Paths.get(a[0]))) {
            int n;
            while ((n = in.read(buf)) != -1) total += n;
        }
        System.out.println(total);
    }
}
