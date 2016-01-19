pub trait Chip8System {
    fn render(&mut self, &[u8; 2048]);

    fn get_input(&mut self) -> Vec<u8>;

    fn is_closed(&mut self) -> bool;
}
