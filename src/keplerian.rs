use std::collections::HashMap;
use std::fmt;
use bevy::prelude::*;
use crate::*;
use bevy::render::mesh::PrimitiveTopology;
use bevy_math::{DMat3, DVec3};
use big_space::prelude::*;
// https://ssd.jpl.nasa.gov/planets/approx_pos.html

#[derive(Clone)]
pub struct OE {
    pub a: f64,
    pub e: f64,
    pub i: f64,
    pub f: f64,
    pub ω: f64,
    pub Ω: f64,
}

impl OE {
    pub fn empty() -> Self {
        OE {
            a: 0.0,
            e: 0.0,
            i: 0.0,
            f: 0.0,
            ω: 0.0,
            Ω: 0.0,
        }
    }
}

impl fmt::Display for OE {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "OE {{ a: {:.6}, e: {:.6}, i: {:.6}, f: {:.6}, arg_per: {:.6}, long_asc: {:.6} }}",
            self.a, self.e, self.i.to_degrees(), self.f.to_degrees(), self.ω.to_degrees(), self.Ω.to_degrees()
        )
    }
}

pub fn position_from_true_anomoly(oe: &OE, f: f64) -> DVec3 {
    let r_mag = oe.a * (1.0 - oe.e.powi(2)) / (1.0 + oe.e * f.cos());
    let r_pqw = DVec3::from_array([r_mag * f.cos(), r_mag * f.sin(), 0.0]);
    let rot = pqw_to_inertial_rot(oe.Ω, oe.ω, oe.i);
    rot * r_pqw
}



pub fn pqw_to_inertial_rot(Ω: f64, ω: f64, i: f64 ) -> DMat3 {
    DMat3::from_cols(
        DVec3::new(
            Ω.cos() * ω.cos() - Ω.sin() * ω.sin() * i.cos(),
            Ω.sin() * ω.cos() + Ω.cos() * ω.sin() * i.cos(),
            ω.sin() * i.sin(),
        ),
        DVec3::new(
            -Ω.cos() * ω.sin() - Ω.sin() * ω.cos() * i.cos(),
            -Ω.sin() * ω.sin() + Ω.cos() * ω.cos() * i.cos(),
            ω.cos() * i.sin(),
        ),
        DVec3::new(
            Ω.sin() * i.sin(),
            -Ω.cos() * i.sin(),
            i.cos(),
        ),
    )
}
pub fn true_anomoly_from_mean_anomaly(e: f64, M: f64) -> f64 {
    let mut E = M;
    for _ in 0..10 {
        let f = E - e * E.sin() - M;
        let f_prime = 1.0 - e * E.cos();
        E -= f / f_prime;
    }

    2.0 * (( (1.0 + e).sqrt() * (E / 2.0).sin() ) / ( (1.0 - e).sqrt() * (E / 2.0).cos() )).atan()
}

pub fn oe_from_rv(mu: f64, body_state: &BodyState) -> OE {
    let r = body_state[0];
    let v = body_state[1];

    let r_mag = r.distance(DVec3::ZERO);
    let v_mag = v.distance(DVec3::ZERO);

    let h = r.cross(v);
    let h_mag = h.distance(DVec3::ZERO);

    let energy = 0.5 * v_mag.powi(2) - mu / r_mag;

    let a = -mu / (2.0 * energy);

    let e_vec = (v.cross(h) / mu) - (r / r_mag);
    let e = e_vec.distance(DVec3::ZERO);

    let i = (h.z / h_mag).acos();

    let k = DVec3::Z;
    let n = k.cross(h);
    let n_mag = n.distance(DVec3::ZERO);

    // Longitude of the ascending node (Ω).
    // If n_norm is very small (i.e. equatorial orbit), set Ω to zero.
    let Ω = if n_mag > 1e-10 {
        let mut Ω_val = (n.x / n_mag).acos();
        if n.y < 0.0 {
            Ω_val = 2.0 * PI - Ω_val;
        }
        Ω_val
    } else {
        0.0
    };

    // Argument of periapsis (ω).
    // Only defined if orbit is non-circular and the node vector exists.
    let ω = if n_mag > 1e-10 && e > 1e-10 {
        let mut ω_val = (n.dot(e_vec) / (n_mag * e)).acos();
        if e_vec.z < 0.0 {
            ω_val = 2.0 * PI - ω_val;
        }
        ω_val
    } else {
        0.0
    };

    // True anomaly (f).
    // For elliptical orbits (non-circular), use the eccentricity vector.
    let f = if e > 1e-10 {
        let mut f_val = (e_vec.dot(r) / (e * r_mag)).acos();
        if r.dot(v) < 0.0 {
            f_val = 2.0 * PI - f_val;
        }
        f_val
    } else {
        // For a circular orbit, the true anomaly is the angle between the node vector and r.
        let mut f_val = (n.dot(r) / (n_mag * r_mag)).acos();
        if r.z < 0.0 {
            f_val = 2.0 * PI - f_val;
        }
        f_val
    };

    OE { a, e, i, f, ω, Ω }
}

pub fn lambert(mu: f64, r1: &DVec3, r2: &DVec3, dt: f64) -> OE {
    let (v1,v2) = lambert_bate::get_velocities(r1.to_array(), r2.to_array(), dt, mu, true, 1e-7, 100).unwrap();
    oe_from_rv(mu, &[*r1,DVec3::from_array(v1)])
}

pub fn oe_to_vec(oe: &OE) -> Vec<Vec3> {
    let mut positions: Vec<Vec3> = Vec::new();
    if oe.a >= 0.0 { // If elliptical
        for i in 0..3600 {
            let theta = (i as f64 / 10.0).to_radians();
            let r = position_from_true_anomoly(&oe, theta);
            positions.push(r.as_vec3());
        }
    } else { // If hyperbolic
        let theta_limit = (-1.0 / oe.e).acos();
        let steps = 3600;
        let dtheta = (theta_limit * 2.0) / steps as f64;

        for i in 10..steps-10 {
            let theta = -theta_limit + dtheta * i as f64;
            let r = position_from_true_anomoly(&oe, theta);
            positions.push(r.as_vec3());
        }
    }

    positions
}