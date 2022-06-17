fn main() {
    let n: i32 = 1;
    let mut n = 0;
    n += 1;

    let s: &str = "hello,world!";
    let mut s = String::from("hello,");
    s.push_str("world!");

    println!("{}", s);

    let mut arr: [i32; 4] = [0, 2, 4, 8];
    arr[0] = -2;
    println!("{}", arr[0] + arr[1]);

    for i in arr.iter() {
        println!("{}", i);
    }

    println!("Hello, world!");
}
