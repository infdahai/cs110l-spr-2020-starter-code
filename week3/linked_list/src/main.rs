use linked_list::LinkedList;

use crate::linked_list::ComputeNorm;
pub mod linked_list;

fn main() {
    let mut list: LinkedList<u32> = LinkedList::new();
    assert!(list.is_empty());
    assert_eq!(list.get_size(), 0);
    for i in 1..12 {
        list.push_front(i);
    }
    println!("{}", list);
    println!("list size: {}", list.get_size());
    println!("top element: {}", list.pop_front().unwrap());
    println!("{}", list);
    println!("size: {}", list.get_size());
    println!("{}", list); // ToString impl for anything impl Display

    for val in &list {
        println!("{}", val);
    }

    let mut list2 = LinkedList::<f64>::new();
    list2.push_front(3.1);
    list2.push_front(2.1);
    println!("compute norm value is {}", list2.compute_norm());
}
