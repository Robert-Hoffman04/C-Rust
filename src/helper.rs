
pub trait strExtensions {
    fn remove_first_and_last(&self) -> &str;
}

impl strExtensions for &str
{
    fn remove_first_and_last(&self) -> &str
    {
        let mut chars = self.chars();
        chars.next();          // Remove first character
        chars.next_back();     // Remove last character
        chars.as_str()         // Return the remaining slice
    }
}