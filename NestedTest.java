public class NestedTest {
    static class Address {
        String city;
        String country;
        
        Address(String city, String country) {
            this.city = city;
            this.country = country;
        }
    }
    
    static class Person {
        String name;
        Address address;
        
        Person(String name, Address address) {
            this.name = name;
            this.address = address;
        }
    }
    
    public static void main(String[] args) throws Exception {
        System.out.println("Starting nested test...");
        Thread.sleep(30000);
        
        Person[] people = {
            new Person("Alice", new Address("NYC", "USA")),
            new Person("Bob", new Address("London", "UK"))
        };
        
        System.out.println("Objects initialized");
        System.out.println("Done");
    }
}
