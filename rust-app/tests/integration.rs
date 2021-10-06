#![cfg_attr(not(target_os = "linux"), no_std)]

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use hex::encode;
    use hyper;
    use hyper::client::connect::HttpConnector;
    use ledger_apdu::APDUCommand;
    use speculos_api::apis;
    use speculos_api::apis::DefaultApi;
    use speculos_api::models::button::*;
    use speculos_api::models::*;
    use std::future::Future;
    use std::sync::atomic::{AtomicBool, Ordering};
    use tokio::process::Command;
    use tokio::test;
    use tokio::time::{sleep, Duration};
    use tokio_retry::strategy::FixedInterval;
    use tokio_retry::Retry;

    use std::env;
    use tokio::sync::Semaphore;

    static DID_BUILD: AtomicBool = AtomicBool::new(false);
    use lazy_static::lazy_static;
    lazy_static! {
        static ref LEDGER_APP: Semaphore = Semaphore::new(1);
    }

    async fn with_speculos<F, Fut, O>(f: F) -> O
    where
        F: Fn(apis::DefaultApiClient<HttpConnector>) -> Fut,
        Fut: Future<Output = O>,
    {
        let speculos_lock = LEDGER_APP.acquire();
        println!("PASSED THE LOCK");

        if !DID_BUILD.load(Ordering::Relaxed) {
            let debug = env::var("DEBUG").unwrap_or_default();
            let features = match debug.as_str() {
                "verbose" => "speculos,extra_debug",
                _ => "speculos",
            };
            eprintln!("Building with {}\n", features);
            match Command::new("cargo")
                .args(["build", "-Z", "build-std=core", "--features", features])
                .status()
                .await
                .map(|s| s.success())
            {
                Ok(true) => (),
                _ => {
                    print!("Build Failed; terminating");
                    std::process::exit(1);
                }
            }
            DID_BUILD.store(true, Ordering::Relaxed);
        }

        let _speculos = Command::new("speculos")
            .args([
                "./target/thumbv6m-none-eabi/debug/rust-app",
                "--display",
                "headless",
            ])
            .kill_on_drop(true)
            .spawn()
            .expect("Failed to execute speculos");

        let raw_client = hyper::client::Client::new();
        let client = apis::DefaultApiClient::new(std::rc::Rc::new(
            apis::configuration::Configuration::new(raw_client),
        ));

        let strat = FixedInterval::from_millis(100);
        match Retry::spawn(strat, || async {
            let a = client.events_delete().await;
            a
        })
        .await
        {
            Ok(_) => {}
            Err(_) => {
                panic!("failed to delete previous events");
            }
        }

        let rv = f(client).await;

        core::mem::drop(speculos_lock);

        rv
    }

    /*#[test]
    async fn run_unit_tests() {
        let debug = env::var("DEBUG").unwrap_or_default();
        let features = match debug.as_str() {
            "verbose" => "speculos,extra_debug",
            _ => "speculos",
        };
        assert_eq!(Some(true), Command::new("cargo")
            .args(["test", "-Z", "build-std=core", "--features", features])
            .status().await.map(|s| s.success()).ok());
    }*/

    #[test]
    async fn test_provide_pubkey() {
        with_speculos(|client| async move {
            let payload = vec!(0x01,0x00,0x00,0x00,0x00);
            let provide_pubkey = APDUCommand {
                cla: 0,
                ins: 2,
                p1: 0,
                p2: 0,
                data: payload
            };

            let res_async = client.apdu_post(Apdu::new(encode(provide_pubkey.serialize())));

            let btns = async {
                sleep(Duration::from_millis(2000)).await;
                client.button_button_post(ButtonName::Right, Button { action: Action::PressAndRelease, delay: Some(0.5) }).await.ok()?;
                client.button_button_post(ButtonName::Right, Button { action: Action::PressAndRelease, delay: Some(0.5) }).await.ok()?;
                client.button_button_post(ButtonName::Both, Button { action: Action::PressAndRelease, delay: Some(0.5) }).await.ok()?;
                Some::<()>(())
            };
            let (res, _) = futures::join!(res_async, btns);

            assert_eq!(res.ok(), Some(Apdu { data: "046f760e57383e3b5900f7c23b78a424e74bebbe9b7b46316da7c0b4b9c2c9301c0c076310eda30506141dd47c2d0a8a1d7ca2542482926ae23b781546193b96169000".to_string() }));
            client.events_delete().await.ok()?;
            Some(())

        }).await;
        ()
    }

    #[test]
    async fn test_provide_pubkey_twice() {
        with_speculos(|client| async move {
            let payload = vec!(0x01,0x00,0x00,0x00,0x00);
            let provide_pubkey = APDUCommand {
                cla: 0,
                ins: 2,
                p1: 0,
                p2: 0,
                data: payload
            };

            let res_async = client.apdu_post(Apdu::new(encode(provide_pubkey.serialize())));

            let btns = async {
                sleep(Duration::from_millis(2000)).await;
                client.button_button_post(ButtonName::Right, Button { action: Action::PressAndRelease, delay: Some(0.5) }).await.ok()?;
                client.button_button_post(ButtonName::Right, Button { action: Action::PressAndRelease, delay: Some(0.5) }).await.ok()?;
                client.button_button_post(ButtonName::Both, Button { action: Action::PressAndRelease, delay: Some(0.5) }).await.ok()?;
                Some(())
            };
            let (res, _) = futures::join!(res_async, btns);

            assert_eq!(res.ok(), Some(Apdu { data: "046f760e57383e3b5900f7c23b78a424e74bebbe9b7b46316da7c0b4b9c2c9301c0c076310eda30506141dd47c2d0a8a1d7ca2542482926ae23b781546193b96169000".to_string() }));

            let payload_2 = vec!(0x02,  0x00,0x00,0x00,0x00,  0x00, 0x01, 0x00, 0x00);
            let provide_pubkey_2 = APDUCommand {
                cla: 0,
                ins: 2,
                p1: 0,
                p2: 0,
                data: payload_2
            };

            let res_async_2 = client.apdu_post(Apdu::new(encode(provide_pubkey_2.serialize())));

            let btns = async {
                sleep(Duration::from_millis(2000)).await;
                client.button_button_post(ButtonName::Right, Button { action: Action::PressAndRelease, delay: Some(0.5) }).await.ok()?;
                client.button_button_post(ButtonName::Right, Button { action: Action::PressAndRelease, delay: Some(0.5) }).await.ok()?;
                client.button_button_post(ButtonName::Both, Button { action: Action::PressAndRelease, delay: Some(0.5) }).await.ok()?;
                Some(())
            };
            let (res_2, _) = futures::join!(res_async_2, btns);

            assert_eq!(res_2.ok(), Some(Apdu { data: "04b90248e0ca25f494e709105e82624145dae654449d81fb557f6b764d1461940080139785d8fc752bb070751f1ef3ff4723119fb6ba1ab14c01a8be8f975311649000".to_string() }));
            client.events_delete().await.ok()?;
            Some(())
        }).await;
        ()
    }

    #[test]
    async fn test_sign() {
        with_speculos(|client| async move {
            let bip32 : Vec<u8> = vec!(0x01,0x00,0x00,0x00,0x00);
            let cmd = br#"
              {
                "payload":{
                  "exec":{
                    "data": null,
                    "code": "(+ 1 2)"
                  }
                },
                "signers":[{
                  "pubKey":"368820f80c324bbc7c2b0610688a7da43e39f91d118732671cd9c7500ff43cca",
                  "caps": ["(accounts.PAY \"alice\" \"bob\" 20.0)"]
                  }],
                "meta":{
                  "gasLimit":1000,
                  "chainId":"0",
                  "gasPrice":1.0e-2,
                  "sender":"sender00"
                  },
                "nonce":"\\\"2019-06-20 20:56:39.509435 UTC\\\"",
                "networkId": "testnet00"
              }"#;
            let payload : Vec<_>= (cmd.len() as u32).to_le_bytes().iter().chain(cmd.iter()).chain(bip32.iter()).cloned().collect();
            // let payload : Vec<_>= cmd.iter().chain(bip32.iter()).cloned().collect();

            let res_async = async {
                let mut res = None;
                for chunk in payload.chunks(230) {

                    let provide_pubkey = APDUCommand {
                        cla: 0,
                        ins: 3,
                        p1: 0,
                        p2: 0,
                        data: chunk.to_vec()
                    };

                    res = Some(client.apdu_post(Apdu::new(encode(provide_pubkey.serialize()))).await);
                }
                res.unwrap()
            };

            let btns = async {
                sleep(Duration::from_millis(2000)).await;
                client.button_button_post(ButtonName::Right, Button { action: Action::PressAndRelease, delay: Some(0.5) }).await.ok()?;
                sleep(Duration::from_millis(2000)).await;
                client.button_button_post(ButtonName::Right, Button { action: Action::PressAndRelease, delay: Some(0.5) }).await.ok()?;
                sleep(Duration::from_millis(2000)).await;
                client.button_button_post(ButtonName::Both, Button { action: Action::PressAndRelease, delay: Some(0.5) }).await.ok()?;
                sleep(Duration::from_millis(2000)).await;
                client.button_button_post(ButtonName::Right, Button { action: Action::PressAndRelease, delay: Some(0.5) }).await.ok()?;
                sleep(Duration::from_millis(2000)).await;
                client.button_button_post(ButtonName::Right, Button { action: Action::PressAndRelease, delay: Some(0.5) }).await.ok()?;
                sleep(Duration::from_millis(2000)).await;
                client.button_button_post(ButtonName::Both, Button { action: Action::PressAndRelease, delay: Some(0.5) }).await.ok()?;

                sleep(Duration::from_millis(2000)).await;
                client.button_button_post(ButtonName::Right, Button { action: Action::PressAndRelease, delay: Some(0.5) }).await.ok()?;
                sleep(Duration::from_millis(2000)).await;
                client.button_button_post(ButtonName::Right, Button { action: Action::PressAndRelease, delay: Some(0.5) }).await.ok()?;
                sleep(Duration::from_millis(2000)).await;
                client.button_button_post(ButtonName::Both, Button { action: Action::PressAndRelease, delay: Some(0.5) }).await.ok()?;
                sleep(Duration::from_millis(2000)).await;
                Some(())
            };
            let (res, _) = futures::join!(res_async, btns);

            assert_eq!(res.ok(), Some(Apdu { data: "304402204a962141bf360df448babd764c0a9553cf63319bbd29817de6bcf9904a4f910802204a349b649d3ba85590b54e8130c28cb9573ffb84e7abc10f72e5f6d1e02b3a159000".to_string() }));

            Some(())
        }).await;
        ()
    }
}
