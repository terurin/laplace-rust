#![feature(test)]
extern crate rayon;
extern crate test;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
const size: usize = 8;

fn main() {
    parallel();
}

fn single() {
    let (dx, dy) = (1. / size as f32, 1. / size as f32);
    let tile = dx * dy;
    let heat = 10.0;
    let eps = 0.001;
    let mut now = vec![vec![0.0; size + 2]; size + 2];
    let mut last = vec![vec![0.0; size + 2]; size + 2];
    let mut count = 0;
    loop {
        for i in 1..size + 1 {
            for j in 1..size + 1 {
                last[i][j] = now[i][j];
            }
        }

        for i in 1..size + 1 {
            for j in 1..size + 1 {
                now[i][j] = (tile * heat
                    + last[i + 1][j]
                    + last[i - 1][j]
                    + last[i][j + 1]
                    + last[i][j - 1])
                    / 4.0;
            }
        }

        count += 1;
        if (count % 100) == 0 {
            let mut sum = 0.0;
            for i in 1..size + 1 {
                for j in 1..size + 1 {
                    let d = last[i][j] - now[i][j];
                    sum += (d * d).sqrt();
                }
            }
            println!("Count={},Error={}", count, sum);
            if sum < eps {
                break;
            }
        }
    }
}

fn parallel() {
    let (dx, dy) = (1. / size as f32, 1. / size as f32);
    let tile = dx * dy;
    let heat = 10.0;
    let eps = 0.001;
    let mut now: Vec<Arc<Mutex<Vec<f32>>>> = Vec::new();
    let mut last_tmp: Vec<Vec<f32>> = Vec::new();
    for _ in 0..size + 2 {
        now.push(Arc::new(Mutex::new(vec![0.0; size + 2])));
        last_tmp.push(vec![0.0; size + 2]);
    }
    let mut last: Arc<Vec<Vec<f32>>> = Arc::new(last_tmp);
    let mut count = 0;
    loop {
        //next step now <- last
        {
            //heat simulater
            let (tx, rx) = mpsc::channel();
            for i in 1..size + 1 {
                let now_row = now[i].clone();
                let last = last.clone();
                let tx = tx.clone();
                thread::spawn(move || {
                    let mut now_row = now_row.lock().unwrap();
                    for j in 1..size + 1 {
                        (*now_row)[j] = (heat * tile
                            + last[i + 1][j + 0]
                            + last[i - 1][j + 0]
                            + last[i + 0][j + 1]
                            + last[i + 0][j - 1])
                            / 4.0;
                    }
                    tx.send(()).unwrap();
                });
            }
            //join session
            for _ in 1..size + 1 {
                rx.recv().unwrap();
            }
        }
        {
            count += 1;
            if (count%100)==0{
                let (tx, rx) = mpsc::channel();
                for i in 1..size + 1 {
                    let now_row = now[i].clone();
                    let last = last.clone();
                    let tx = tx.clone();
                    thread::spawn(move || {
                        let now_row = now_row.lock().unwrap();
                        let mut sum = 0.0;
                        for j in 1..size + 1 {
                            let d = last[i][j] - (*now_row)[j];
                            sum += (d * d).sqrt();
                        }
                        tx.send(sum).unwrap();
                    });
                }
                let mut sum = 0.0;
                for _ in 1..size + 1 {
                    sum += rx.recv().unwrap();
                }
                println!("Count={},Error={}", count, sum);
                if sum < eps {
                    break;
                }
            }
        }
        //copy session
        {
            let mut last_tmp: Vec<Vec<f32>> = Vec::new();
            for i in 0.. size+2{
                let now_row=now[i].clone();
                let now_row=now_row.lock().unwrap();
                last_tmp.push(now_row.to_vec());
            }
            last=Arc::new(last_tmp);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test::Bencher;

    #[bench]
    fn test_single(b: &mut Bencher) {
        b.iter(|| single());
    }

    #[bench]
    fn test_parallel(b: &mut Bencher) {
        b.iter(|| parallel());
    }
}
