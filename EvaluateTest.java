public class EvaluateTest {
    public static void main(String[] args) throws Exception {
        System.out.println("Starting... waiting for debugger to set breakpoint");
        Thread.sleep(30000);  // 30 seconds to set breakpoint
        
        int x = 42;
        String message = "Hello, World!";
        double pi = 3.14159;
        
        System.out.println("x = " + x);
        System.out.println("message = " + message);
        System.out.println("pi = " + pi);
        
        // Breakpoint here to test evaluate
        System.out.println("Done");
    }
}
