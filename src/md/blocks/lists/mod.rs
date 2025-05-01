pub mod bullet_list;
pub mod list_item;
pub mod ordered_list;

#[derive(Debug)]
pub enum List<State> {
    Ordered(ordered_list::OrderedList<State>),
    Bullet(bullet_list::BulletList<State>),
}
