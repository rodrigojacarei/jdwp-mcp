import java.util.ArrayList;
import java.util.HashMap;

public class MethodTest {
    public static void main(String[] args) throws Exception {
        System.out.println("Starting method test...");
        Thread.sleep(30000);
        
        ArrayList<String> list = new ArrayList<>();
        list.add("first");
        list.add("second");
        list.add("third");
        
        HashMap<String, Integer> map = new HashMap<>();
        map.put("age", 30);
        map.put("count", 5);
        
        String text = "Hello World";
        
        System.out.println("Objects initialized");
        System.out.println("Done");
    }
}
