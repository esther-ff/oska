pub mod bullet_list;
pub mod list_item;
pub mod ordered_list;

#[derive(Debug)]
pub enum List {
    Ordered(ordered_list::OrderedList),
    Bullet(bullet_list::BulletList),
}
