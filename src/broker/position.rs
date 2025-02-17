pub struct Position {
    pub quantity: f64,
    pub average_price: f64,
}

impl Position {
    pub fn new(quantity: f64, price: f64) -> Self {
        Position {
            quantity,
            average_price: price,
        }
    }

    pub fn update(&mut self, quantity: f64, price: f64) {
        let total_cost = self.average_price * self.quantity + price * quantity;
        self.quantity += quantity;
        self.average_price = total_cost / self.quantity;
    }

    pub fn remove(&mut self, quantity: f64) -> Result<(), String> {
        if quantity > self.quantity {
            return Err("Cannot remove more than the available quantity.".to_string());
        }
        self.quantity -= quantity;
        Ok(())
    }
}
