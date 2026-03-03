public class StepTest {
    public static void main(String[] args) throws Exception {
        System.out.println("Starting step test...");
        Thread.sleep(30000);  // 30 seconds
        
        int a = 10;
        int b = 20;
        int sum = add(a, b);
        System.out.println("Sum: " + sum);
        
        int product = multiply(a, b);
        System.out.println("Product: " + product);
        
        System.out.println("Done");
    }
    
    public static int add(int x, int y) {
        return x + y;
    }
    
    public static int multiply(int x, int y) {
        return x * y;
    }
}
