import java.util.*;
import java.math.*;

public class JdkClassTest {
    public static void main(String[] args) throws Exception {
        System.out.println("Starting JDK class test...");
        Thread.sleep(30000);
        
        // Collections
        ArrayList<String> arrayList = new ArrayList<>();
        arrayList.add("item");
        
        LinkedList<String> linkedList = new LinkedList<>();
        linkedList.add("item");
        
        HashMap<String, Integer> hashMap = new HashMap<>();
        hashMap.put("key", 1);
        
        TreeMap<String, Integer> treeMap = new TreeMap<>();
        treeMap.put("key", 1);
        
        HashSet<String> hashSet = new HashSet<>();
        hashSet.add("item");
        
        // Wrappers
        Integer intObj = 42;
        Long longObj = 100L;
        Double doubleObj = 3.14;
        Boolean boolObj = true;
        
        // Other common types
        StringBuilder sb = new StringBuilder("test");
        StringBuffer sbuf = new StringBuffer("test");
        BigInteger bigInt = new BigInteger("12345");
        BigDecimal bigDec = new BigDecimal("123.45");
        
        System.out.println("Objects initialized");
        System.out.println("Done");
    }
}
