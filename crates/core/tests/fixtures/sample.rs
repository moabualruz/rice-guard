use std::collections::HashMap;
use std::fmt;

pub struct MyStruct {
    value: i32,
}

impl MyStruct {
    pub fn compute(&self) -> i32 {
        let a = self.value * 2;
        let b = a + 10;
        let c = b - 5;
        c
    }

    pub fn display(&self) {
        println!("{}", self.value);
    }
}

pub fn standalone_function(x: i32, y: i32) -> i32 {
    x + y
}
