use rayon;

pub use crate::steal;

pub trait Benchable<'a, R>  : Send + Sync{
    fn start(&mut self) -> ();
    fn name(&self) -> &'static str;
    // fn id(&self) -> BenchmarkId;
    fn verify(&self, _result: &R) -> bool { return true }
    fn get_result(&self) -> R;
    fn get_thread_pool(&self, num_threads: usize, backoffs: Option<usize>) -> rayon::ThreadPool{
        let mut pool = rayon::ThreadPoolBuilder::new().num_threads(num_threads);
        if let Some(backoffs) = backoffs {
            pool = pool.steal_callback(move |x| steal::steal(backoffs, x));
        }
        pool.build().unwrap()
    }
    fn reset(&mut self) {}
}
