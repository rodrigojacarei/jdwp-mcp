public class DeepTest {
    static class Extra {
        String info;
        Extra(String info) {
            this.info = info;
        }
    }
    
    static class Address {
        String city;
        Extra extra;
        
        Address(String city, Extra extra) {
            this.city = city;
            this.extra = extra;
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
        System.out.println("Starting deep test...");
        Thread.sleep(30000);
        
        Person[] people = {
            new Person("Alice", new Address("NYC", new Extra("details1"))),
            new Person("Bob", new Address("London", new Extra("details2")))
        };
        
        System.out.println("Objects initialized");
        System.out.println("Done");
    }
}
