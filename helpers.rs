use bevy::prelude::*;

pub fn ra_dec_to_vec(ra: f32, dec: f32) -> DVec3 {
    let dec = dec.to_radians();
    let ra = ra.to_radians();
    x = dec.cos() * rad.cos();
    y = dec.cos() * rad.sin();
    z = dec.sin();

    DVec3::new(x, y, z)
}