public class ObjectTest {
    static class Person {
        String name;
        int age;
        
        Person(String name, int age) {
            this.name = name;
            this.age = age;
        }
    }
    
    public static void main(String[] args) throws Exception {
        System.out.println("Starting object test...");
        Thread.sleep(30000);
        
        Person person = new Person("Alice", 30);
        Person[] people = {
            new Person("Bob", 25),
            new Person("Charlie", 35)
        };
        
        System.out.println("Objects initialized");
        System.out.println("Done");
    }
}
