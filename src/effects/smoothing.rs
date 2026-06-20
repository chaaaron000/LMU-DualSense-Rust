#[derive(Debug, Clone)]
pub struct Smoother {
    value: f32,
    attack: f32,
    release: f32,
}

impl Smoother {
    pub fn new(attack: f32, release: f32) -> Self {
        Self {
            value: 0.0,
            attack,
            release,
        }
    }

    pub fn update(&mut self, target: f32) -> f32 {
        let target = target.clamp(0.0, 1.0);
        let alpha = if target > self.value {
            self.attack
        } else {
            self.release
        };
        self.value += (target - self.value) * alpha;
        self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uses_attack_and_release_rates() {
        let mut smoother = Smoother::new(0.5, 0.25);
        assert_eq!(smoother.update(1.0), 0.5);
        assert_eq!(smoother.update(0.0), 0.375);
    }
}
