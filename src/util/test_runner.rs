macro_rules! run_test {
    ($test_name:ident, $($param:tt)*) => {
        {
            let result = $test_name($($param)*).await;
            let status = match result {
                Ok(_) => "PASSED".to_owned(),
                Err(error) => {
                    if let crate::util::error::TestError::Texture(texture_error) = &error {
                        crate::util::image::save_image_async(stringify!($test_name), &texture_error.texture).await?;
                    }
                    format!("FAILED - {}", error)
                },
            };
            println!("{}: {}", stringify!($test_name), status);
        }
    }
}
