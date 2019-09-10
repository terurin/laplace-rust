#![feature(test)]
extern crate rayon;
extern crate test;
extern crate threadpool;
use std::sync::{mpsc, Arc, Mutex, RwLock};
use std::thread;
use threadpool::ThreadPool;
const ncpus: usize = 8;
const size: usize = 64;
const eps: f32 = 0.001;
fn main() {
    //single();
    parallel();
}

fn single() {
    let (dx, dy) = (1. / size as f32, 1. / size as f32);
    let tile = dx * dy;
    let heat = 10.0;
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
    let heat = 10.0;
    let mut now: Vec<Arc<RwLock<Vec<f32>>>> = Vec::new();
    let mut last: Vec<Arc<RwLock<Vec<f32>>>> = Vec::new();
    for _ in 0..size + 2 {
        now.push(Arc::new(RwLock::new(vec![0.0; size + 2])));
        last.push(Arc::new(RwLock::new(vec![0.0; size + 2])));
    }
    let mut count = 0;
    let pool = ThreadPool::new(ncpus);
    loop {
        //last <- now
        {
            let (tx, rx) = mpsc::channel();
            for i in 1..size + 1 {
                let last_row = last[i].clone();
                let now_row = now[i].clone();
                let tx = tx.clone();
                pool.execute(move || {
                    let mut last_row = last_row.write().unwrap();
                    let now_row = now_row.read().unwrap();
                    for j in 1..size + 1 {
                        last_row[j] = now_row[i];
                    }
                    tx.send(()).unwrap();
                });
            }
            for _ in 1..size + 1 {
                rx.recv().unwrap();
            }
        }
        //next step now <- last
        {
            //heat simulater
            let (tx, rx) = mpsc::channel();
            for i in 1..size + 1 {
                let last_up = last[i - 1].clone();
                let last_mid = last[i].clone();
                let last_down = last[i + 1].clone();
                let now_row = now[i].clone();
                let tx = tx.clone();
                pool.execute(move || {
                    let last_up = last_up.read().unwrap();
                    let last_mid = last_mid.read().unwrap();
                    let last_down = last_down.read().unwrap();
                    let mut now_row = now_row.write().unwrap();
                    for j in 1..size + 1 {
                        now_row[j] = (heat * dx * dy
                            + last_up[j + 0]
                            + last_mid[j - 1]
                            + last_mid[j + 1]
                            + last_down[j + 0])
                            / 4.0;
                    }
                    tx.send(()).unwrap();
                });
            }

            for _ in 1..size + 1 {
                rx.recv().unwrap();
            }
        }

        count += 1;
        if (count % 100) == 0 {
            let (tx, rx) = mpsc::channel();
            for i in 1..size + 1 {
                let last_row = last[i].clone();
                let now_row = now[i].clone();
                let tx = tx.clone();
                pool.execute(move || {
                    let last_row = last_row.read().unwrap();
                    let now_row = now_row.read().unwrap();
                    let mut error = 0.0;
                    for j in 1..size + 1 {
                        let d = last_row[j] - now_row[i];
                        error += (d * d).sqrt();
                    }
                    tx.send(error).unwrap();
                });
            }
            let mut error = 0.0;
            for _ in 0..size {
                error += rx.recv().unwrap();
            }
            println!("Count={},Error={}", count, error);
            if error < eps {
                break;
            }
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
