extern crate image;
use std::path::Path;
use itertools::izip;
use std::env;
use image::{ImageBuffer, Rgb,Luma};
use image::imageops::FilterType;
use std::time::Instant;
use std::collections::VecDeque;
use std::fs::File;
use std::fs;
// Overall O'notation of 4n(ish) (n being image size=width*height)
// I think that's pretty good.

const B_SPACING:usize = 20usize; // Border space
const WHITE_SPACE_SYMBOL:char = ' '; // What symbol to use when priting white pixels
const LUMA_ADJUSTMENT:u8 = 15u8; // Luma less than avg_luma-LUMA_ADJUSTMENT set to 0 or greater than set to 255.


// O(3n + some shit)
fn main() {
    let args: Vec<String> = env::args().collect();
    let img_name = &args[1];

    println!("img_name: {}",img_name);

    let path = &format!("images/{}",img_name);
    let path = Path::new(path);

    let img = image::open(path).unwrap().to_luma();
    
    let dims = img.dimensions();
    let (width,height) = (dims.0 as usize,dims.1 as usize);
    
    let mut img_raw:Vec<u8> = img.clone().into_raw();
    
    // 2d vector of size of image, where each pixel will be labelled as to which symbol it belongs
    let mut symbols:Vec<Vec<u32>> = to_bin2dvec(width,height,&mut img_raw);

    // Debug stuff to check binarisation worked:
    let check_img:ImageBuffer<Luma<u8>,Vec<u8>> = ImageBuffer::<Luma<u8>,Vec<u8>>::from_raw(width as u32,height as u32,img_raw).unwrap();
    check_img.save("check_img.png").unwrap();

    // Printing can be nice to visualize process.
    // But for larger images it simply prints useless spam in the console.
    if width <= 200 && height <= 400 {
        symbols_intial_prt(&symbols);
    }
    flood_segmentation(&path,width,height,&mut symbols);
}

#[allow(dead_code,non_snake_case)]
fn prt_u8_vec__as_2d((width,height):(usize,usize),vec:&Vec<u8>) -> () {
    println!();
    let shape = (width,height); // shape[0],shape[1]=row,column
    let spacing = 4*shape.0;
    println!("┌ {:<1$}┐","─",spacing);
    for row in 0..shape.1 {
        print!("│ ");
        for col in 0..shape.0 {
            print!("{} ",vec[row*width+col]);
            
        }
        println!("│");
    }
    println!("└ {:<1$}┘","─",spacing);
    print!("{:<1$}","",(spacing/2)-1);
    println!("[{},{}]",shape.0,shape.1);
    println!();
}

fn flood_segmentation(path:&Path,width:usize,height:usize,symbols:&mut Vec<Vec<u32>>) {
    let start = Instant::now();
    let mut symbol_count = 2u32;
    let mut pixels_in_symbols:Vec<Vec<(usize,usize)>> = Vec::new();
    let start_flood = Instant::now();
    for y in 0..height {
        for x in 0..width {
            if symbols[y][x] == 0 {
                pixels_in_symbols.push(Vec::new());
                let last_index = pixels_in_symbols.len()-1;
                flood_fill_queue(symbols,symbol_count,width,height,x,y,&mut pixels_in_symbols[last_index]);
                symbol_count += 1;
            }
        }
    }
    println!("{} : Flood finished",time(start_flood));
    
    if width <= 200 && height <= 400 {
        symbols_classified_prt(&symbols);
    }

    // Set borders of symbols
    let (borders,bounds) = set_borders(symbols,&pixels_in_symbols,width,height,path);
    // Create symbol images
    let mut symbol_images:Vec<ImageBuffer<Luma<u8>,Vec<u8>>> = create_symbol_images(&pixels_in_symbols,&borders,width,height);
    // Export symbol images
    for i in 0..symbol_images.len() {
        let path = format!("split/{}.png",i);
        let mut scaled_image = image::imageops::resize(&mut symbol_images[i],35,35,FilterType::Triangle);
        image::imageops::colorops::invert(&mut scaled_image);
        scaled_image.save(path).unwrap();
    }
    //Export bounds
    let bounds_str = String::from(format!("{:?}",bounds));
    fs::write("splitInfo.txt",bounds_str).unwrap();
    println!("{} : Flood segmented",time(start));

    fn flood_fill_queue(symbols:&mut Vec<Vec<u32>>,symbol_count:u32,width:usize,height:usize,x:usize,y:usize,pixels:&mut Vec<(usize,usize)>) {
        pixels.push((x,y));
        symbols[y][x] = symbol_count;
        let mut queue:VecDeque<(usize,usize)> = VecDeque::new();
        queue.push_back((x,y));
        loop {
            if let Some(n) = queue.pop_front() {
                // +x
                if n.0 < width-1 {
                    let (x,y) = (n.0+1,n.1);
                    if symbols[y][x] == 0 {
                        pixels.push((x,y));
                        symbols[y][x] = symbol_count;
                        queue.push_back((x,y));
                    }
                }
                // -x
                if n.0 > 0 {
                    let (x,y) = (n.0-1,n.1);
                    if symbols[y][x] == 0 {
                        pixels.push((x,y));
                        symbols[y][x] = symbol_count;
                        queue.push_back((x,y));
                    }
                }
                // +y
                if n.1 < height-1 {
                    let (x,y) = (n.0,n.1+1);
                    if symbols[y][x] == 0 {
                        pixels.push((x,y));
                        symbols[y][x] = symbol_count;
                        queue.push_back((x,y));
                    }
                }
                // -y
                if n.1 > 0 {
                    let (x,y) = (n.0,n.1-1);
                    if symbols[y][x] == 0 {
                        pixels.push((x,y));
                        symbols[y][x] = symbol_count;
                        queue.push_back((x,y));
                    }
                }
            }
            else { break; } // If queue empty break
        }
    }
    fn set_borders(symbols:&mut Vec<Vec<u32>>,pixel_symbols:&Vec<Vec<(usize,usize)>>,width:usize,height:usize,path:&Path) -> (Vec<((usize,usize),(usize,usize))>,Vec<((usize,usize),(usize,usize))>) {
        let start = Instant::now();
        // Gets bounds
        let mut border_bounds:Vec<((usize,usize),(usize,usize))> = Vec::new();
        let mut bounds:Vec<((usize,usize),(usize,usize))> = Vec::new();
        for symbol in pixel_symbols {
            let mut lower_x = symbol.iter().fold(width, |min,x| (if x.0 < min { x.0 } else { min }));
            let mut lower_y = symbol.iter().fold(height, |min,x| (if x.1 < min { x.1 } else { min }));
            let mut upper_x = symbol.iter().fold(0usize, |max,x| (if x.0 > max { x.0 } else { max }));
            let mut upper_y = symbol.iter().fold(0usize, |max,x| (if x.1 > max { x.1 } else { max }));

            bounds.push(((lower_x,lower_y),(upper_x,upper_y)));

            if lower_x >= B_SPACING { lower_x -= B_SPACING; };
            if lower_y >= B_SPACING { lower_y -= B_SPACING; };
            if upper_x + B_SPACING < width { upper_x += B_SPACING; };
            if upper_y + B_SPACING < height { upper_y += B_SPACING; };

            border_bounds.push(((lower_x,lower_y),(upper_x,upper_y)));
        }
        // Copies image
        let mut border_img = image::open(path).unwrap().into_rgb();
        for (x,y,pixel) in border_img.enumerate_pixels_mut() {
            let val = if symbols[y as usize][x as usize] == 1 { 255 } else { 0 };
            *pixel = image::Rgb([val,val,val]);
        }

        // Sets borders
        let border_pixel = image::Rgb([255,0,0]); // Pixel to use as border
        for symbol in border_bounds.iter() {
            let min_x = (symbol.0).0;
            let min_y = (symbol.0).1;
            let max_x = (symbol.1).0;
            let max_y = (symbol.1).1;
            // Sets horizontal borders
            for i in min_x..max_x {
                *border_img.get_pixel_mut(i as u32,min_y as u32) = border_pixel;
                *border_img.get_pixel_mut(i as u32,max_y as u32) = border_pixel;
            }
            // Sets vertical borders
            for i in min_y..max_y {
                *border_img.get_pixel_mut(min_x as u32,i as u32) = border_pixel;
                *border_img.get_pixel_mut(max_x as u32,i as u32) = border_pixel;
            }
            // Sets bottom corner border
            *border_img.get_pixel_mut(max_x as u32,max_y as u32) = border_pixel;
        }
        border_img.save("borders.png").unwrap();
        println!("{} : Borders set",time(start));
        return (border_bounds,bounds);
    }
    fn create_symbol_images(pixels_in_symbols:&Vec<Vec<(usize,usize)>>,borders:&Vec<((usize,usize),(usize,usize))>,width:usize,height:usize) -> Vec<ImageBuffer<Luma<u8>,Vec<u8>>> {
        let start = Instant::now();
        let sizes:Vec<(usize,usize)> = borders.iter().map(|lims| ((lims.1).0-(lims.0).0+1,(lims.1).1-(lims.0).1+1)).collect();
        // Default constructs to all black pixels, thus we set symbol pixels to white in following loop
        // TODO Look into constructing with default white pixels and drawing black pixels
        let mut symbol_images:Vec<ImageBuffer<Luma<u8>,Vec<u8>>> = sizes.iter().map(|size| ImageBuffer::<Luma<u8>,Vec<u8>>::new(size.0 as u32,size.1 as u32)).collect();
        // O(n)
        // Draws symbol images
        for i in 0..pixels_in_symbols.len() {
            let offset:(usize,usize) = borders[i].0;
            for pixel in pixels_in_symbols[i].iter() {
                let x = pixel.0;
                let y = pixel.1;
                let out_pixel = symbol_images[i].get_pixel_mut((x-offset.0) as u32,(y-offset.1) as u32);
                *out_pixel = image::Luma([255]);
            }
        }
        println!("{} : Created symbol images",time(start));
        return symbol_images;
    }
}

// Converts img raw into binary image which it returns as a 2d vector.
fn to_bin2dvec(width:usize,height:usize,img_raw:&mut Vec<u8>) -> Vec<Vec<u32>> {
    // 2d vector of size of image, where each pixel will be labelled as to which symbol it belongs
    let start = Instant::now();
    let mut symbols:Vec<Vec<u32>> = vec!(vec!(1u32;width as usize);height as usize);
    println!("width * height = length : {} * {} = {}|{}k|{}m",width,height,img_raw.len(),img_raw.len()/1000,img_raw.len()/1000000);
    
    let avg_luma:u8 = (img_raw.iter().fold(0u32,|sum,x| sum+*x as u32) / img_raw.len() as u32) as u8;
    println!("avg_luma:{}",avg_luma);

    for y in 0..height {
        for x in 0..width {
            let luma = img_raw[y*width+x];
            img_raw[y*width+x] = if luma < avg_luma-LUMA_ADJUSTMENT { 0 } else { 255 };
            symbols[y][x] = (img_raw[y*width+x] / 255) as u32;
        }
    }
    println!("{} : Converted image to binary",time(start));
    return symbols;
}

fn time(instant:Instant) -> String {
    let mut millis = instant.elapsed().as_millis();
    let seconds = (millis as f32 / 1000f32).floor();
    millis = millis % 1000;
    let time = format!("{:#02}:{:#03}",seconds,millis);
    return time;
}

// Nicely prints Vec<Vec<u8>> as matrix
#[allow(dead_code,non_snake_case)]
pub fn symbols_intial_prt(matrix:&Vec<Vec<u32>>) -> () {

    println!();
    let shape = (matrix.len(),matrix[0].len()); // shape[0],shape[1]=row,column
    let spacing = 1*shape.0;
    horizontal_number_line(shape.0);

    println!("    ┌{:─<1$}┐","",spacing);
    for row in 0..shape.1 {
        vertical_number_line(row);

        print!("│");
        for col in 0..shape.0 {
            if matrix[col][row] == 1 { print!("{}",matrix[col][row]);/*print!("{}",WHITE_SPACE_SYMBOL);*/ }
            else { print!("{}",matrix[col][row]); }
            
            
        }
        println!("│");
    }
    println!("    └{:─<1$}┘","",spacing);
    print!("   {:<1$}","",(spacing/2)-1);
    println!("   [{},{}]",shape.0,shape.1);
    println!();

    fn horizontal_number_line(rows:usize) -> () {
        print!("\n   ");
        for col in 0..rows/10 {
            print!("{: <1$}","",4);
            print!("{: >2}",col);
            print!("{: <1$}","",4);
        }
        print!("\n    ");
        for _ in 0..rows/10 {
            print!("┌{:─<1$}┐","",8);
        }
        print!("┌{:─<1$}","",rows%10);
        print!("\n    ");
        for col in 0..rows {
            print!("{: >1}",col%10)
        }
        println!();
    }

    fn vertical_number_line(row:usize) -> () {
        if row % 10 == 5 {
            print!("{: >2}",row/ 10);
        } else { print!("  "); }

        if row % 10 == 0 {
            print!("┌");
        }
        else if row % 10 == 9 {
            print!("└");
        }
        else {
            print!("│");
        }
        print!("{}",row % 10);
    }
}
#[allow(dead_code,non_snake_case)]
pub fn symbols_classified_prt(matrix:&Vec<Vec<u32>>) -> () {

    println!();
    let shape = (matrix.len(),matrix[0].len()); // shape[0],shape[1]=row,column
    let spacing = 2*shape.0;
    
    horizontal_number_line(shape.0);

    println!("    ┌─{:─<1$}┐","",spacing);
    for row in 0..shape.1 {
        vertical_number_line(row);

        print!("│");
        for col in 0..shape.0 {
            // TODO Do the whitespace print better
            if matrix[col][row] == 1 { print!(" {}",WHITE_SPACE_SYMBOL); }
            else { print!("{: >2}",matrix[col][row]); }
        }
        println!(" │");
    }
    println!("    └─{:─<1$}┘","",spacing);
    print!("{:<1$}","",(spacing/2)-1);
    println!("[{},{}]",shape.0,shape.1);
    println!();

    fn horizontal_number_line(rows:usize) -> () {
        print!("\n   ");
        for col in 0..rows/10 {
            print!("{: <1$}","",9);
            print!("{: >2}",col);
            print!("{: <1$}","",9);
        }
        print!("\n    ");
        for _ in 0..rows/10 {
            print!("┌{:─<1$}┐","",2*9);
        }
        print!("┌{:─<1$}","",rows%10);
        print!("\n    ");
        for col in 0..rows {
            print!("{: >2}",col%10)
        }
        println!();
    }

    fn vertical_number_line(row:usize) -> () {
        if row % 10 == 5 {
            print!("{: >2}",row/ 10);
        } else { print!("  "); }

        if row % 10 == 0 {
            print!("┌");
        }
        else if row % 10 == 9 {
            print!("└");
        }
        else {
            print!("│");
        }
        print!("{}",row % 10);
    }
}
