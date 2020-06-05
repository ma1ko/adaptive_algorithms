use crate::adaptive_bench::Benchable;
pub use crate::task::Task;
use rand::Rng;

#[derive(Debug, PartialEq)]
pub struct Point {
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
    pub fn create_random_points(size: usize) -> Vec<Point> {
        let mut rng = rand::thread_rng();

        std::iter::repeat_with(|| Point::new(rng.gen::<f64>(), rng.gen::<f64>()))
            .take(size)
            .collect::<Vec<Point>>()
    }
}

pub struct Searcher<'a> {
    points: &'a [Point],
    start_index: usize,
    end_index: usize,
    min: f64,
}

impl<'a> Searcher<'a> {
    pub fn new(points: &'a [Point]) -> Searcher {
        Searcher {
            points,
            start_index: 0,
            end_index: points.len(),
            min: 100.0,
        }
    }
}

impl<'a> Benchable<'a, f64> for Searcher<'a> {
    fn start(&mut self) {
        self.run_();
    }
    fn name(&self) -> &'static str {
        "adaptive_point_search"
    }
    fn verify(&self, result: &f64) -> bool {
        *result == self.min
    }
    fn get_result(&self) -> f64 {
        self.min
    }
    fn reset(&mut self) {
        *self = Searcher::new(&self.points);
    }
}

impl<'a> Task for Searcher<'a> {
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

pub struct RayonPoints<'a> {
    points: &'a [Point],
    min: f64,
}
impl<'a> RayonPoints<'a> {
    pub fn new(points: &'a [Point]) -> Self {
        RayonPoints { points, min: 100.0 }
    }
}
use rayon::prelude::*;

impl<'a> Benchable<'a, f64> for RayonPoints<'a> {
    fn start(&mut self) {
        let iter = self
            .points
            .par_iter()
            .enumerate()
            .map(|(i, a)| {
                let inner_iter = self.points[i + 1..].iter().map(|b| a.distance_to(b));
                inner_iter.fold(1.0f64, |x, y| x.min(y))
            })
            .collect::<Vec<f64>>();
        let min = iter.iter().fold(1.0f64, |x, y| x.min(*y));
        self.min = min
    }
    fn verify(&self, result: &f64) -> bool {
        self.min == *result
    }
    fn name(&self) -> &'static str {
        "Nearest Points Rayon"
    }
    fn get_result(&self) -> f64 {
        self.min
    }
    fn reset(&mut self) {
        self.min = 100.0;
    }
}
