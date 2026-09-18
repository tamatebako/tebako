public class Fib {
    public static long fib(int n) { return n < 2 ? n : fib(n-1) + fib(n-2); }
    public static void main(String[] a) { System.out.println(fib(32)); }
}
