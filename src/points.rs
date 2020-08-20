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
    fn start(&mut self) -> Option<f64> {
        *self = Searcher::new(&self.points);
        self.run();
        Some(self.min)
    }
    fn name(&self) -> &'static str {
        "Adaptive"
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

        t.run_with(self);
        self.min = self.min.min(t.min);
        self.start_index = (self.start_index + 1).min(self.end_index);
    }
    fn can_split(&self) -> bool {
        return self.end_index - self.start_index > 1;
    }
    fn split(&mut self, mut runner: impl FnMut(&mut Vec<&mut Self>), steal_counter: usize) {
        // let half = (self.end_index - self.start_index) / 2 + self.start_index;
        let mut start_index = self.start_index;
        let end_index = self.end_index;
        // how many elements per task? We need at least one
        let mut step = (end_index - start_index) / (steal_counter + 1) + 1;
        if end_index - start_index <= 2 {
            step = 1;
        }
        let mut tasks = vec![];
        self.end_index = start_index + step;
        start_index += step;
        while start_index < end_index {
            let other: Searcher<'a> = Searcher {
                points: self.points,
                start_index: start_index.min(end_index),
                end_index: (start_index + step).min(end_index),
                min: self.min,
            };
            tasks.push(other);
            start_index += step;
        }
        let mut tasks = tasks.iter_mut().collect::<Vec<&mut Self>>();
        tasks.insert(0, self);
       // println!("{:?}", tasks.iter().map(|x| (x.start_index, x.end_index)).collect::<Vec<_>>());
        runner(&mut tasks);
    }

    fn is_finished(&self) -> bool {
        assert!(self.end_index >= self.start_index);
        self.end_index == self.start_index
    }
    fn fuse(&mut self, other: &mut Self) {
        self.min = self.min.min(other.min);
    }
    fn work(&self) -> Option<(&'static str, usize)> {
        Some(("First Level", self.end_index - self.start_index))
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
        let end_index = (self.start_index + 1024).min(self.end_index);
        let others = &self.points[self.start_index..end_index];
        for other in others {
            min = min.min(point.distance_to(other));
        }
        self.min = min;
        self.start_index = end_index;
    }
    fn can_split(&self) -> bool {
        self.end_index - self.start_index > 10000
        // false
    }
    fn split(&mut self, mut runner: impl FnMut(&mut Vec<&mut Self>), _steal_counter: usize) {
        let half = (self.end_index - self.start_index) / 2 + self.start_index;
        let mut other: Tester<'a> = Tester {
            points: self.points,
            point: self.point,
            start_index: half,
            end_index: self.end_index,
            min: self.min,
        };
        self.end_index = half;
        runner(&mut vec![self, &mut other]);
    }
    fn is_finished(&self) -> bool {
        self.end_index == self.start_index
    }
    fn fuse(&mut self, other: &mut Self) {
        self.min = self.min.min(other.min);
    }
    // We can't do a subtask here, it'll just be too much for rayon_logs to handle
    // fn work(&self) -> Option<(&'static str, usize)> {
    //     Some(("Second Level", self.start_index - self.end_index))
    // }
}

pub struct RayonPoints<'a> {
    points: &'a [Point],
}
impl<'a> RayonPoints<'a> {
    pub fn new(points: &'a [Point]) -> Self {
        RayonPoints { points }
    }
}
use rayon::prelude::*;

impl<'a> Benchable<'a, f64> for RayonPoints<'a> {
    fn start(&mut self) -> Option<f64> {
        let iter = self
            .points
            .par_iter()
            .enumerate()
            .map(|(i, a)| {
                let inner_iter = self.points[i + 1..].iter().map(|b| a.distance_to(b));
                inner_iter.fold(1.0f64, |x, y| x.min(y))
            })
            .collect::<Vec<f64>>();
        Some(iter.iter().fold(1.0f64, |x, y| x.min(*y)))
    }
    fn name(&self) -> &'static str {
        "Rayon"
    }
}

pub struct FlatMapPoints<'a> {
    points: &'a [Point],
}
impl<'a> FlatMapPoints<'a> {
    pub fn new(points: &'a [Point]) -> Self {
        FlatMapPoints { points }
    }
}
impl<'a> Benchable<'a, f64> for FlatMapPoints<'a> {
    fn start(&mut self) -> Option<f64> {
        let points = self.points.clone();
        let iter = points
            .par_iter()
            .enumerate()
            .flat_map(|(i, a)| {
                let inner_iter = self.points[i + 1..]
                    .par_iter()
                    .map(move |b| a.distance_to(b));
                // inner_iter.fold(1.0f64, |x, y| x.min(y))
                inner_iter
            })
            .reduce(|| 1.0f64, |x, y| x.min(y));
        Some(iter)
    }
    fn name(&self) -> &'static str {
        "FlatMap"
    }
}
