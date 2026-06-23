use crate::defs::ElementType;

pub fn type_modifier(attack: ElementType, defend: ElementType) -> u16 {
    match (attack, defend) {
        (ElementType::Electric, ElementType::Water) => 200,
        (ElementType::Electric, ElementType::Grass) => 50,
        (ElementType::Fire, ElementType::Grass) => 200,
        (ElementType::Water, ElementType::Fire) => 200,
        (ElementType::Grass, ElementType::Water) => 200,
        (ElementType::Grass, ElementType::Fire) => 50,
        (ElementType::Water, ElementType::Grass) => 50,
        (ElementType::Fire, ElementType::Water) => 50,
        _ => 100,
    }
}
