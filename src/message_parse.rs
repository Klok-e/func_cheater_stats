use lazy_static::lazy_static;
use regex;

lazy_static! {
    static ref IS_SOLUTION_REGEX: regex::Regex =
        regex::Regex::new(r"^\d[\s\S]*?https://pastebin\.com/").unwrap();
    static ref KATA_KYU: regex::Regex = regex::Regex::new(r"^\d(?:\s*kyu|\s)").unwrap();
    static ref JUST_LINK: regex::Regex =
        regex::Regex::new(r"https://pastebin\.com/[a-zA-Z\d]*").unwrap();
    static ref LINK_AND_EVERYTHING_AFTER: regex::Regex =
        regex::Regex::new(r"https://pastebin\.com/(.|\s)*").unwrap();
}

pub fn is_codewars_solution(msg: &str) -> bool {
    IS_SOLUTION_REGEX.is_match(msg)
}

pub fn kata_name_link(msg: &str) -> (String, String) {
    if !is_codewars_solution(msg) {
        panic!("Text {} is not a codewars solution", msg);
    }
    let link = JUST_LINK
        .find(msg)
        .expect(format!("Link not found in {}", msg).as_str());
    let name = LINK_AND_EVERYTHING_AFTER.replace(msg, "");
    (
        name.trim().replace("\n", " "),
        link.as_str().trim().replace("\n", " "),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kata_name_link_test1() {
        let message = "7
Functions of Integers on Cartesian Plane
https://pastebin.com/nRkGjfp5";

        assert!(is_codewars_solution(message));

        assert_eq!(
            kata_name_link(message),
            (
                "7 Functions of Integers on Cartesian Plane".to_owned(),
                "https://pastebin.com/nRkGjfp5".to_owned()
            )
        )
    }

    #[test]
    fn kata_name_link_test2() {
        let message = "7
Robinson Crusoe
https://pastebin.com/fZHdUbhT";

        assert!(is_codewars_solution(message));

        assert_eq!(
            kata_name_link(message),
            (
                "7 Robinson Crusoe".to_owned(),
                "https://pastebin.com/fZHdUbhT".to_owned()
            )
        )
    }

    #[test]
    fn kata_name_link_test3() {
        let message = "6
Replace With Alphabet Position
https://pastebin.com/8hPWe1L6";

        assert!(is_codewars_solution(message));

        assert_eq!(
            kata_name_link(message),
            (
                "6 Replace With Alphabet Position".to_owned(),
                "https://pastebin.com/8hPWe1L6".to_owned()
            )
        )
    }

    #[test]
    fn kata_name_link_test4() {
        let message = "6
Create Phone Number
https://pastebin.com/grekUgAs";

        assert!(is_codewars_solution(message));

        assert_eq!(
            kata_name_link(message),
            (
                "6 Create Phone Number".to_owned(),
                "https://pastebin.com/grekUgAs".to_owned()
            )
        )
    }
}
