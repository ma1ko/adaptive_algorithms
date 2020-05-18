extern crate rand;
use mergesort::task::Task;
use rand::Rng;
use rayon::prelude::*;
use std::time::Instant;
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

pub fn main() {
    println!("Running");

    let points = create_random_points(50000);
    let now = Instant::now();

    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap();
    let min = pool.install(|| {
        let iter = points
            .par_iter()
            .enumerate()
            .map(|(i, a)| {
                let inner_iter = points[i + 1..].iter().map(|b| a.distance_to(b));
                inner_iter.fold(1.0f64, |x, y| x.min(y))
            })
            .collect::<Vec<f64>>();
        let min = iter.iter().fold(1.0f64, |x, y| x.min(*y));
        min
    });
    let new_now = Instant::now();
    println!("{:?}", new_now.duration_since(now));
    println!("Closest points have a distance of {}", min);

    println!("My Algo");

    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .steal_callback(|x| mergesort::steal::steal(6, x))
        .build()
        .unwrap();
    let mut s = Searcher {
        points: &points,
        start_index: 0,
        end_index: points.len(),
        min: 100.0, // FLOAT_MAX
    };
    pool.install(|| s.run_());
    let now = Instant::now();
    assert_eq!(min, s.min);
    println!("{:?}", now.duration_since(new_now));
    println!("My result: {}", s.min);
}

struct Tester<'a> {
    points: &'a [Point],
    start_index: usize,
    end_index: usize,
    min: f64,
    point: &'a Point,
}

impl<'a> Task for Tester<'a> {
    fn step(&mut self) {
        let mut min = self.min;
        let point = self.point;
        let end_index = (self.start_index + 128).min(self.end_index);
        let others = &self.points[self.start_index..end_index];
        for other in others {
            min = min.min(point.distance_to(other));
        }
        self.min = min;
        self.start_index = end_index;
    }
    fn can_split(&self) -> bool {
        self.end_index - self.start_index > 1024
    }
    fn split(&mut self) -> Self {
        let half = (self.end_index - self.start_index) / 2 + self.start_index;
        let other: Tester<'a> = Tester {
            points: self.points,
            point: self.point,
            start_index: half,
            end_index: self.end_index,
            min: self.min,
        };
        self.end_index = half;
        other
    }
    fn is_finished(&self) -> bool {
        self.end_index == self.start_index
    }
    fn fuse(&mut self, other: Self) {
        self.min = self.min.min(other.min);
    }
}

struct Searcher<'a> {
    points: &'a [Point],
    start_index: usize,
    end_index: usize,
    min: f64,
}

impl<'a> mergesort::task::Task for Searcher<'a> {
    fn step(&mut self) {
        let mut t = Tester {
            points: self.points,
            start_index: self.start_index + 1,
            end_index: self.points.len(),
            min: self.min,
            point: &self.points[self.start_index],
        };
        t.run(Some(self));
        self.min = self.min.min(t.min);
        self.start_index = (self.start_index + 1).min(self.end_index);
    }
    fn can_split(&self) -> bool {
        return self.end_index - self.start_index > 16;
    }

    fn split(&mut self) -> Self {
        let half = (self.end_index - self.start_index) / 2 + self.start_index;
        let other: Searcher<'a> = Searcher {
            points: self.points,
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
