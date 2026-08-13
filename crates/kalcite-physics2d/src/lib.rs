#![no_std]

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Aabb {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Circle {
    pub x: i32,
    pub y: i32,
    pub radius: i32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Motion {
    pub dx: i32,
    pub dy: i32,
}

/// Normal components use 10 fractional bits (1024 == 1.0).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Contact {
    pub normal_x: i32,
    pub normal_y: i32,
    pub penetration: i32,
}

pub fn hit(a: Aabb, b: Aabb) -> bool {
    a.x < b.x.saturating_add(b.w)
        && a.x.saturating_add(a.w) > b.x
        && a.y < b.y.saturating_add(b.h)
        && a.y.saturating_add(a.h) > b.y
}

pub fn circle_hit(a: Circle, b: Circle) -> bool {
    let dx = i64::from(b.x) - i64::from(a.x);
    let dy = i64::from(b.y) - i64::from(a.y);
    let radii = i64::from(a.radius.max(0).saturating_add(b.radius.max(0)));
    dx * dx + dy * dy < radii * radii
}

pub fn circle_contact(a: Circle, b: Circle) -> Option<Contact> {
    let dx = i64::from(b.x) - i64::from(a.x);
    let dy = i64::from(b.y) - i64::from(a.y);
    let radii = i64::from(a.radius.max(0).saturating_add(b.radius.max(0)));
    let distance_sq = dx * dx + dy * dy;
    if distance_sq >= radii * radii {
        return None;
    }
    if distance_sq == 0 {
        return Some(Contact {
            normal_x: 1024,
            normal_y: 0,
            penetration: radii.min(i64::from(i32::MAX)) as i32,
        });
    }
    let distance = i64::from(integer_sqrt(distance_sq as u64).max(1));
    Some(Contact {
        normal_x: (dx * 1024 / distance) as i32,
        normal_y: (dy * 1024 / distance) as i32,
        penetration: (radii - distance).min(i64::from(i32::MAX)) as i32,
    })
}

/// Separates two equal-mass circles and applies an impulse.
/// `restitution` is expressed as a percentage in the 0..=100 range.
pub fn resolve_circles(
    a: &mut Circle,
    velocity_a: &mut Motion,
    b: &mut Circle,
    velocity_b: &mut Motion,
    restitution: u8,
) -> bool {
    let Some(contact) = circle_contact(*a, *b) else {
        return false;
    };
    let correction = (contact.penetration + 1) / 2;
    let correction_x = contact.normal_x.saturating_mul(correction) / 1024;
    let correction_y = contact.normal_y.saturating_mul(correction) / 1024;
    a.x = a.x.saturating_sub(correction_x);
    a.y = a.y.saturating_sub(correction_y);
    b.x = b.x.saturating_add(correction_x);
    b.y = b.y.saturating_add(correction_y);

    let relative_x = velocity_b.dx.saturating_sub(velocity_a.dx);
    let relative_y = velocity_b.dy.saturating_sub(velocity_a.dy);
    let separating_velocity = (relative_x.saturating_mul(contact.normal_x)
        + relative_y.saturating_mul(contact.normal_y))
        / 1024;
    if separating_velocity >= 0 {
        return true;
    }
    let impulse =
        (-separating_velocity).saturating_mul(100 + i32::from(restitution.min(100))) / 200;
    let impulse_x = contact.normal_x.saturating_mul(impulse) / 1024;
    let impulse_y = contact.normal_y.saturating_mul(impulse) / 1024;
    velocity_a.dx = velocity_a.dx.saturating_sub(impulse_x);
    velocity_a.dy = velocity_a.dy.saturating_sub(impulse_y);
    velocity_b.dx = velocity_b.dx.saturating_add(impulse_x);
    velocity_b.dy = velocity_b.dy.saturating_add(impulse_y);
    true
}

/// Swept, pixel-stepped AABB movement. This cannot tunnel through a thin wall.
pub fn move_and_slide(mut body: Aabb, motion: Motion, solids: &[Aabb]) -> (Aabb, Motion) {
    let applied_x = move_axis(&mut body, motion.dx, true, solids);
    let applied_y = move_axis(&mut body, motion.dy, false, solids);
    (
        body,
        Motion {
            dx: applied_x,
            dy: applied_y,
        },
    )
}

fn move_axis(body: &mut Aabb, amount: i32, horizontal: bool, solids: &[Aabb]) -> i32 {
    let direction = amount.signum();
    let mut applied = 0i32;
    for _ in 0..amount.unsigned_abs() {
        let mut next = *body;
        if horizontal {
            next.x = next.x.saturating_add(direction);
        } else {
            next.y = next.y.saturating_add(direction);
        }
        if solids.iter().copied().any(|solid| hit(next, solid)) {
            break;
        }
        *body = next;
        applied = applied.saturating_add(direction);
    }
    applied
}

fn integer_sqrt(value: u64) -> u32 {
    if value < 2 {
        return value as u32;
    }
    let mut x = value;
    let mut y = (x + 1) / 2;
    while y < x {
        x = y;
        y = (x + value / x) / 2;
    }
    x.min(u64::from(u32::MAX)) as u32
}

#[cfg(test)]
mod tests {
    extern crate std;
    use super::*;

    #[test]
    fn blocks_motion_without_tunnelling() {
        let body = Aabb {
            x: 0,
            y: 0,
            w: 8,
            h: 8,
        };
        let wall = Aabb {
            x: 10,
            y: 0,
            w: 1,
            h: 8,
        };
        let (next, applied) = move_and_slide(body, Motion { dx: 20, dy: 3 }, &[wall]);
        assert_eq!(next.x, 2);
        assert_eq!(applied.dx, 2);
        assert_eq!(next.y, 3);
    }

    #[test]
    fn reports_circle_contact() {
        let contact = circle_contact(
            Circle {
                x: 0,
                y: 0,
                radius: 5,
            },
            Circle {
                x: 8,
                y: 0,
                radius: 5,
            },
        )
        .unwrap();
        assert_eq!(contact.normal_x, 1024);
        assert_eq!(contact.normal_y, 0);
        assert_eq!(contact.penetration, 2);
    }

    #[test]
    fn resolves_circle_overlap_and_velocity() {
        let mut a = Circle {
            x: 0,
            y: 0,
            radius: 5,
        };
        let mut b = Circle {
            x: 8,
            y: 0,
            radius: 5,
        };
        let mut velocity_a = Motion { dx: 2, dy: 0 };
        let mut velocity_b = Motion { dx: -2, dy: 0 };
        assert!(resolve_circles(
            &mut a,
            &mut velocity_a,
            &mut b,
            &mut velocity_b,
            80
        ));
        assert!(b.x - a.x >= 10);
        assert!(velocity_a.dx < 0);
        assert!(velocity_b.dx > 0);
    }
}
