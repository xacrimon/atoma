use flize::Collector;

const ITER: usize = 1 << 14;

fn reclaim_single_thread() {
    let collector = Collector::new();

    for i in 0..ITER {
        let shield = collector.shield();
        
        for _ in 0..ITER {
            shield.retire(|| ());
        }

        collector.collect();
        dbg!(i);
    }
}

fn main() {
    reclaim_single_thread();
}
