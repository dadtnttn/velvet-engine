use velvet_pixel::sdf::{circle, union};
use velvet_pixel::{hex, Surface};

#[test]
fn hex_colours_read_as_rgb() {
    assert_eq!(hex("#ff8020"), [255.0, 128.0, 32.0]);
    assert_eq!(hex("0a0610"), [10.0, 6.0, 16.0]);
}

#[test]
fn adding_light_saturates_and_rounds_half_to_even() {
    let mut s = Surface::new(2, 2);
    s.set_i(1, 1, [250.0, 10.0, 0.0], 1.0);
    s.add(1.0, 1.0, [20.0, 2.5, 0.5], 1.0);
    let i = (s.w + 1) * 4;
    // 270 clamps to 255; 12.5 rounds to 12 (even), 0.5 to 0, as the JavaScript's Uint8ClampedArray
    assert_eq!(&s.data[i..i + 3], &[255, 12, 0]);
}

#[test]
fn a_union_is_inside_where_either_field_is() {
    let body = union(vec![
        circle(0.0, 0.0, 2.0, 1.0),
        circle(10.0, 0.0, 2.0, 1.0),
    ]);
    assert!(body(0.0, 0.0).0 < 0.0);
    assert!(body(10.0, 0.0).0 < 0.0);
    assert!(body(5.0, 0.0).0 > 0.0);
}
