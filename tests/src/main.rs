#[cfg(test)]
mod tests {
    use oska::lexer_again;

    // #[test]
    // fn create_lexer_only_text() {
    //     let text = concat!(
    //         "nom de dieu\n",
    //         "ahahahahaha\n",
    //         "europe\n\n",
    //         "newblock\n",
    //         "test"
    //     );

    //     let mut lexer = lexer::MdLexer::new(text).expect("failed to create lexer");

    //     lexer.lex();

    //     // dbg!(lexer.root());
    // }

    #[test]
    fn bolds() {
        let text = concat!(
            "*nom de dieu*\n",
            "***\n",
            "---\n",
            "    HAH!\n",
            "___\n",
            "*-*\n",
            "_ahaha_\n",
            "__aaaaaa__\n",
            "**bbbbbbb**\n",
            "***bbbbbbb***\n",
            "___bbbbbbb___\n",
            "europe\n\n",
            "newblock\n",
            "test"
        );

        println!("{}", text);

        let lexer = lexer_again::Lexer::new(text.as_bytes().to_vec());

        lexer.start();
    }
}
