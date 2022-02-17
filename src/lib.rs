
mod Get402 {

    use bitcoin::util::key::{PrivateKey, PublicKey};
    use bitcoin::util::address::{Address};
    use secp256k1::{Secp256k1};
    use k256::{
        ecdsa::{SigningKey, Signature, signature::Signer}, SecretKey
    };
    use serde::{Serialize, Deserialize};
    use reqwest;
    use reqwest::header::HeaderMap;
    use std::collections::HashMap;
    use uuid::Uuid;
    use base64;
    use bitcoin_hashes::sha256;
    use std::env;

    pub struct KeyPair {

        identifier: String,

        private_key: PrivateKey

    }

    static API_BASE: &str = "https://get402.com/api";
    //static API_BASE: &str = "http://localhost:3000/api";

    impl KeyPair {

        pub fn generate() -> KeyPair {

            let secp = Secp256k1::new();

            let mut rng = rand::rngs::OsRng::new().expect("OsRng");

            let (secret_key, public_key) = secp.generate_keypair(&mut rng);

            let serialized_private_key = secret_key.serialize_secret();

            let serialized_public_key = public_key.serialize();

            let network = bitcoin::network::constants::Network::Bitcoin;

            let private_key = PrivateKey::from_slice(&serialized_private_key, network).unwrap();

            let public_key = PublicKey::from_slice(&serialized_public_key).unwrap();

            let identifier = Address::p2pkh(&public_key, network).to_string();

            KeyPair {

                private_key, identifier

            }

        }

    }

    pub struct App {

        identifier: String,

        private_key: PrivateKey,

    }

    impl App {

        pub fn generate() -> App {

            let key_pair = KeyPair::generate();

            App { 

                private_key: key_pair.private_key,

                identifier: key_pair.identifier,
            }

        }

        pub fn load(private_key_string: &str) -> App {

             let secp = bitcoin::secp256k1::Secp256k1::new();

            let private_key = PrivateKey::from_wif(private_key_string).unwrap();

            let public_key = private_key.public_key(&secp);

            let network = bitcoin::network::constants::Network::Bitcoin;

            let identifier = Address::p2pkh(&public_key, network).to_string();

            App { 

                private_key: private_key,

                identifier: identifier

            }

        }

        pub fn create_client(&self) -> Client {

            let key_pair = KeyPair::generate();

            Client {

                private_key: Some(key_pair.private_key),

                identifier: key_pair.identifier,

                app: self

            }

        }

        pub fn get_client_from_identifier(&self, identifier: &str) -> Client {

            Client {

                private_key: None,

                identifier: identifier.to_string(),

                app: self

            }

        }

    }

    pub struct Client<'a> {

        pub identifier: String,

        private_key: Option<PrivateKey>,

        app: &'a App

    }

    #[derive(Serialize, Deserialize)]
    struct GetBalanceAPIResponse {
        app_id: String,
        client_id: String,
        balance: u64
    }

    #[derive(Serialize, Deserialize, Debug)]
    struct Output {
        script: String,
        amount: u64
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub struct PaymentRequired {
        outputs: Vec<Output>,
        network: String,
        memo: String,
        pub paymentUrl: String,
    }

    #[derive(Debug)]
    pub enum APIError {
        Unauthorized,
        InternalServerError,
        InsufficientFunds(PaymentRequired),
        NotFound,
    }

    #[derive(Serialize, Deserialize)]
    pub struct ChargeCreditAPIResponse {
        app_id: String,
        client_id: String,
        balance: u64
    }

    #[derive(Serialize, Deserialize)]
    struct AuthMessage {
        nonce: String,
        domain: String,
    }

    pub type ChargeCreditResult = Result<ChargeCreditAPIResponse, APIError>;

    impl Client<'_> {

        pub async fn get_balance(&self) -> Result<u64, APIError> {

            let client = reqwest::Client::new();

            let url = format!("{}/apps/{}/clients/{}", API_BASE, self.identifier, self.app.identifier);

            let response = client
                .get(url)
                .send()
                .await
                .unwrap();

            let response = response.json::<GetBalanceAPIResponse>().await.unwrap();

            Ok(response.balance)

        }

        pub async fn request_buy_credits(&self, credits: u64) -> Result<PaymentRequired, APIError> {

            let client = reqwest::Client::new();

            let url = format!("{}/apps/{}/clients/{}/buy-credits/{}", API_BASE, self.identifier, self.app.identifier, credits);

            println!("BUY CREDITS URL {}", url);

            let response = client
                .get(url)
                .send()
                .await
                .unwrap();

            let response = response.json::<PaymentRequired>().await.unwrap();

            Ok(response)

        }

        pub async fn charge_credit(&self, map: &HashMap<&str, u64>) -> ChargeCreditResult {

            let client = reqwest::Client::new();

            let url = format!("{}/apps/{}/clients/{}/calls", API_BASE, self.identifier, self.app.identifier);

            let headers = self.authorize();

            let response = client
                .post(url)
                .headers(headers)
                .json(&map)
                .send()
                .await
                .unwrap();

            match response.status() {
                reqwest::StatusCode::OK => {

                    let response = response.json::<ChargeCreditAPIResponse>().await.unwrap();

                    Ok(response)

                }
                reqwest::StatusCode::UNAUTHORIZED => {
                    println!("charge_credit API call unauthorized");

                    Err(APIError::Unauthorized)
                }
                reqwest::StatusCode::PAYMENT_REQUIRED => {
                    println!("charge_credit API call payment required");

                    let response = response.json::<PaymentRequired>().await.unwrap();

                    Err(APIError::InsufficientFunds(response))

                }
                other => {
                    println!("charge_credit failed with unknown error");
                    Err(APIError::InternalServerError)
                }

            }

        }

        /*fn sign_message(&self, message: &String) -> Signature {

            let secp = Secp256k1::new();

            let private_key_bytes = self.app.ecdsa_private_key;

            let hash = sha256::Hash(message.as_bytes());

            println!("HASH {}", hash)

            let message = secp256k1::Message::from_hashed_data::<sha256::Hash>(message.as_bytes());

            let signature = secp.sign_ecdsa(&message, &self.app.ecdsa_private_key);

            //let signing_key = SigningKey::from_bytes(&private_key_bytes).unwrap();

            //signing_key.sign(message.as_bytes())
        }
        */

        fn authorize(&self) -> HeaderMap {

            let mut headers = HeaderMap::new();

            let nonce = Uuid::new_v4().to_string();

            let auth_message = AuthMessage {
                nonce,
                domain: "get402.com".to_string()
            };

            let message = serde_json::to_string(&auth_message).unwrap();

            //let signature = self.sign_message(&message);
            let signature = "";

            headers.insert("auth-identifier", self.identifier.parse().unwrap());
            headers.insert("auth-message", message.parse().unwrap());
            headers.insert("auth-signature", signature.to_string().parse().unwrap());

            headers

        }

    }


}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn it_generates_new_app_with_random_private_key_and_identifier() {

        let app: Get402::App = Get402::App::generate();

    }

    #[test]
    fn it_loads_an_existing_app_from_privatekey_string() {

        let private_key: &str = "Kzvu3L6wXPWPuscaFBtyJKWYMAHxKtEXKu4VqSbrbgqy6aqoGDL8";

        let app: Get402::App = Get402::App::load(private_key);

    }

    #[test]
    fn it_gets_a_client_from_an_existing_identifier() {

        let private_key: &str = "Kzvu3L6wXPWPuscaFBtyJKWYMAHxKtEXKu4VqSbrbgqy6aqoGDL8";

        let identifier: &str = "12nitHbpWTaDHxNfgLq9E5gWjtWwcgwJn7";

        let app: Get402::App = Get402::App::load(private_key);

        let client: Get402::Client = app.get_client_from_identifier(identifier);

        assert_eq!(client.identifier, identifier.to_string());

    }

    #[tokio::test]
    async fn it_gets_the_balance_of_a_new_api_client_key() {

        let app: Get402::App = Get402::App::generate();

        let client: Get402::Client = app.create_client();

        let balance: u64 = client.get_balance().await.unwrap();

        assert_eq!(balance, 0)

    }
    #[tokio::test]
    #[ignore]
    async fn it_should_get_402_when_charging_a_client_key_with_zero_balance() {

        let app: Get402::App = Get402::App::generate();

        let client: Get402::Client = app.create_client();

        let mut map = HashMap::new();
        map.insert("credits", 1);

        match client.charge_credit(&map).await {

            Err(Get402::APIError::InsufficientFunds(payment_request)) => {
                println!("MATCHED INSUFFICIENT FUNDS!");
            }

            other => {
               println!("Incorrect!");
               assert!(false, "Should have been Insufficient Funds"); 
            }
        }

    }
    #[tokio::test]
    async fn it_gets_a_payment_request_for_more_credits() {

        let app: Get402::App = Get402::App::generate();

        let client: Get402::Client = app.create_client();

        match client.request_buy_credits(10).await {

            Ok(PaymentRequired) => {

                assert_eq!(PaymentRequired.paymentUrl, "https://get402.com/api/payments".to_string());

            }

            other => {
               assert!(false, "Should have been Insufficient Funds"); 
            }
        }
    }
    #[test]
    fn it_sends_a_payment_for_more_credits() {
    }
}
