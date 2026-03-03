public class ArrayTest {
    public static void main(String[] args) throws Exception {
        System.out.println("Starting array test...");
        Thread.sleep(30000);
        
        int[] numbers = {1, 2, 3, 4, 5};
        double[] decimals = {1.1, 2.2, 3.3};
        boolean[] flags = {true, false, true};
        String[] words = {"hello", "world"};
        Object[] mixed = {"text", 42, null};
        
        System.out.println("Arrays initialized");
        System.out.println("Done");
    }
}
