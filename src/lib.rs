extern crate rand;
use rand::Rng;
use rayon::prelude::*;
use std::time::{ Instant};
#[derive(Debug, PartialEq)]
struct Point {
    x: f64,
    y: f64,
}

impl Point {
    pub fn new(x: f64, y: f64) -> Point {
        Point { x, y }
    }

    pub fn distance_to(&self, other: &Point) -> f64 {
        ((self.x - other.x).powi(2) + (self.y - other.y).powi(2)).sqrt()
    }
}

fn create_random_points(size: usize) -> Vec<Point> {
    let mut rng = rand::thread_rng();

    std::iter::repeat_with(|| Point::new(rng.gen::<f64>(), rng.gen::<f64>()))
        .take(size)
        .collect::<Vec<Point>>()
}

use mergesort::task::Task;
pub fn main() {
    println!("Running");

    let points = create_random_points(50000);
    let now = Instant::now();

    let iter = points.par_iter().enumerate().filter_map(|(i, a)| {
        let inner_iter = points[i + 1..].par_iter().map(|b| a.distance_to(b));
        inner_iter.min_by(|x, y| x.partial_cmp(y).unwrap())
    });
    let min = iter.min_by(|x, y| x.partial_cmp(y).unwrap()).unwrap();
    let new_now = Instant::now();
    println!("{:?}", new_now.duration_since(now));
    println!("Closest points have a distance of {}", min);


    println!("My Algo");

    

    let pool = mergesort::rayon::get_thread_pool();
        let mut s = Searcher {
        points: &points,
        start_index: 0,
        end_index: points.len(),
        min : 100.0, // FLOAT_MAX
    };
    pool.install(||
        s.run_()
    );
    let now = Instant::now();
    assert_eq!(min, s.min);
    println!("{:?}", now.duration_since(new_now));
    println!("My result: {}", s.min);


}


struct Searcher<'a> {
    points: &'a [Point],
    start_index: usize,
    end_index: usize,
    min: f64
}
impl<'a> Searcher<'a> {
    fn test(&mut self, index: usize){
        let point = &self.points[index];
        let others = &self.points[index + 1 .. self.points.len()];
        for other in others {
            self.min = self.min.min(point.distance_to(other));
        }

    }
}

impl<'a> mergesort::task::Task for Searcher<'a> {

    fn step(&mut self) {
        let end_index = self.end_index.min(self.start_index + 1);
        // println!("Testing {} to {}", self.start_index, s
        for index in self.start_index .. end_index {
            self.test(index);

        }
        self.start_index = end_index;
        
    }
    fn can_split(&self) -> bool{
       return self.end_index - self.start_index > 128 
    }

    fn split(&mut self) -> Self {
        let half = (self.end_index - self.start_index) / 2 + self.start_index;
        let other : Searcher<'a> = Searcher {
            points: self.points ,
            start_index: half,
            end_index: self.end_index,
            min: self.min,
        };
        self.end_index = half;
        other
    }

    fn is_finished(&self) -> bool {
        assert!(self.end_index >= self.start_index);
        self.end_index == self.start_index

    }
    fn fuse(&mut self, other: Self) {
        self.min = self.min.min(other.min);
    }
}

