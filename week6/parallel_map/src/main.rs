use core::num;
use crossbeam_channel;
use std::{thread, time};

fn parallel_map<T, U, F>(mut input_vec: Vec<T>, num_threads: usize, f: F) -> Vec<U>
where
    F: FnOnce(T) -> U + Send + Copy + 'static,
    T: Send + 'static,
    U: Send + 'static + Default,
{
    let mut output_vec: Vec<U> = Vec::with_capacity(input_vec.len());
    output_vec.resize_with(input_vec.len(), Default::default);
    let mut thread = Vec::new();

    let (in_sender, in_receiver) = crossbeam_channel::unbounded();
    let (out_sender, out_receiver) = crossbeam_channel::unbounded();
    for _ in 0..num_threads {
        let in_receiver = in_receiver.clone();
        let out_sender = out_sender.clone();
        thread.push(thread::spawn(move || {
            while let Ok(next_pair) = in_receiver.recv() {
                // receive the value, handle it, then send it back
                let (idx, value) = next_pair;
                out_sender
                    .send((idx, f(value)))
                    .expect("there is no receiver");
            }
        }));
    }
    let len = input_vec.len();
    for i in 0..len {
        in_sender
            .send((len - i - 1, input_vec.pop().unwrap()))
            .expect("there is no receiver");
    }

    drop(in_sender);
    drop(out_sender);
    while let Ok(result_pair) = out_receiver.recv() {
        let (idx, value) = result_pair;
        // println!("main: {}", idx);
        output_vec[idx] = value;
    }
    for t in thread {
        t.join().expect("Panic occurred in thread");
    }
    output_vec
}

fn main() {
    let v = vec![6, 7, 8, 9, 10, 1, 2, 3, 4, 5, 12, 18, 11, 5, 20];
    let squares = parallel_map(v, 10, |num| {
        println!("{} squared is {}", num, num * num);
        thread::sleep(time::Duration::from_millis(500));
        num * num
    });
    println!("squares: {:?}", squares);
}
