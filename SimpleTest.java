public class SimpleTest {
    public static void main(String[] args) throws Exception {
        Thread.sleep(100); // Small delay to allow breakpoint to be set
        int x = 10;
        int y = 20;
        int sum = x + y;
        System.out.println("Sum: " + sum);
    }
}
