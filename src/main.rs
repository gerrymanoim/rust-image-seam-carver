extern crate clap;
extern crate image;
use clap::{App, Arg};
use std::collections::HashSet;

use image::{Rgb, RgbImage};

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
struct PixelLocation {
    x: u32,
    y: u32,
}

type Seam = HashSet<PixelLocation>;

#[derive(Clone)]
struct SeamPixel {
    energy: f32,
    location: PixelLocation,
    previous: Option<PixelLocation>,
}
type SeamEnergies = Vec<Vec<SeamPixel>>;

type EnergyMap = Vec<Vec<f32>>;

fn get_pixel_energy(left: Option<&Rgb<u8>>, middle: &Rgb<u8>, right: Option<&Rgb<u8>>) -> f32 {
    // TODO refactor

    let left_energy = match left {
        Some(n) => {
            (n[0] as i32 - middle[0] as i32).pow(2)
                + (n[1] as i32 - middle[1] as i32).pow(2)
                + (n[2] as i32 - middle[2] as i32).pow(2)
        }
        _ => 0,
    };
    let right_energy = match right {
        Some(n) => {
            (n[0] as i32 - middle[0] as i32).pow(2)
                + (n[1] as i32 - middle[1] as i32).pow(2)
                + (n[2] as i32 - middle[2] as i32).pow(2)
        }
        _ => 0,
    };

    return (left_energy as f32 + right_energy as f32).sqrt();
}

fn calculate_energy_map(img: &RgbImage) -> EnergyMap {
    // TODO can we just do a convolution here instead with filter3x3

    let mut energy_map : EnergyMap = vec![vec![0.0; img.width() as usize]; img.height() as usize];
    for y in 0..img.height() {
        for x in 1..img.width() {
            let left = Some(img.get_pixel(x - 1, y));
            let middle = img.get_pixel(x, y);
            let right = if x == img.width() {
                None
            } else {
                Some(img.get_pixel(x, y))
            };

            energy_map[y as usize][x as usize] = get_pixel_energy(left, middle, right);
        }
    }
    energy_map
}

fn find_low_energy_seam(energy_map: EnergyMap) -> Seam {
    // TODO this function sucks
    // TODO most of these for loops should be iters on rows or cols?
    let width = energy_map[0].len();
    let height = energy_map.len();
    // TODO this is dumb
    let dummy = SeamPixel{energy: f32::INFINITY, location: PixelLocation{x:0, y:0}, previous: None};
    let mut seam_energies : SeamEnergies = vec![vec![dummy; width]; height];
    // Populate the first row
    for x in 0..width {
        seam_energies[0][x] = SeamPixel {
            energy: energy_map[0][x],
            location: PixelLocation { x: x as u32, y: 0 },
            previous: None,
        };
    }

    // Populate the rest of the rows
    for y in 1..height {
        for x in 0..width {
            let mut min_seen_energy = f32::INFINITY;
            let mut min_prev_x = x;

            // TODO this underflow panics because we're subtracting 1 from a usize at 0
            // index better
            if x  == 0 {
                for i in x..x + 1 {
                    if i > 0 && i < width && seam_energies[y - 1][i].energy < min_seen_energy {
                        min_seen_energy = seam_energies[y - 1][i].energy;
                        min_prev_x = i;
                    }
                }
            } else {
                for i in x - 1..x + 2 {
                    if i > 0 && i < width && seam_energies[y - 1][i].energy < min_seen_energy {
                        min_seen_energy = seam_energies[y - 1][i].energy;
                        min_prev_x = i;
                    }
                }
            }
            seam_energies[y][x] = SeamPixel {
                energy: min_seen_energy + energy_map[y][x],
                location: PixelLocation {
                    x: x as u32,
                    y: y as u32,
                },
                previous: Some(PixelLocation {
                    x: min_prev_x as u32,
                    y: y as u32 - 1,
                }),
            };
        }
    }

    // Find the tail of the lowest energy seam
    let (min_x, _) = seam_energies[height - 1]
        .iter()
        .enumerate()
        .min_by(|(_, l), (_, r)| l.energy.partial_cmp(&r.energy).unwrap())
        .unwrap();

    // Walk up to assemple the seam
    let mut seam = Seam::new();
    let mut current_seam = &seam_energies[height - 1][min_x];
    loop {
        seam.insert(current_seam.location);
        current_seam = match current_seam.previous {
            Some(loc) => &seam_energies[loc.y as usize][loc.x as usize],
            None => break,
        }
    }

    seam
}

fn delete_seam(img: RgbImage, seam: Seam) -> RgbImage {
    // TODO fix this
    let (old_width, old_height) = img.dimensions();
    let pixels: Vec<&Rgb<u8>> = img
        .enumerate_pixels()
        .filter(|(x, y, _)| !seam.contains(&PixelLocation { x: *x, y: *y }))
        .map(|(_, _, pixel)| pixel)
        .collect();

    // TODO this is dumb but I can't figure out how to do it otherwise
    let mut new_img = RgbImage::new(old_width - 1, old_height);
    for it in new_img.enumerate_pixels_mut().zip(pixels.iter()) {
        let ((_, _, dest_pixel), src_pixel) = it;
        *dest_pixel = **src_pixel;
    }

    new_img
}

fn resize_image(img: RgbImage, trim_width: u32) -> RgbImage {
    let mut out = img.clone();
    let (in_width, _) = img.dimensions();
    assert!(in_width > trim_width, "Only resize down supported");

    for _ in 0..trim_width {
        let energy_map = calculate_energy_map(&out);
        let seam = find_low_energy_seam(energy_map);
        out = delete_seam(out, seam)
    }

    out
}

fn main() {
    // TODO: Learn more about clap
    let matches = App::new("Rust Image Seam Caver")
        .version("0.1")
        .arg(Arg::with_name("image").required(true).index(1))
        .arg(Arg::with_name("trim_width").required(true).index(2))
        .get_matches();
    let image_file = matches.value_of("image").unwrap();
    let trim_width = matches
        .value_of("trim_width")
        .unwrap()
        .parse::<u32>()
        .unwrap();

    let img = image::open(image_file).unwrap().to_rgb8();
    let out_imge = resize_image(img, trim_width);

    out_imge.save("out.jpg").unwrap();

    println!("Input file: {}", image_file);
}
