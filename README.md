
# Get402 Client Rust

Build Paid APIs with Ease. This library simplifies offering of APIs that can be accessed only via micropayments

## Installation

```
cargo install get402

```

## Usage
See https://get402.com/docs.html for complete documentation

You may import the entire library or load only specific objects as needed

```

use Get402;


```

### Authentication

Your Get402 API is identified by a public/private key pair where the public address is used to identify your API and the
private key is used to sign requests to get402.com.

#### Using Existing API Private Key


```
let private_key = env::var("GET402_API_PRIVATE_KEY").unwrap();

let app: Get402::App = Get402::App::load(private_key);


```

#### Generating A New API PrivateKey

```

let app: Get402::App = Get402::App::generate();


```

One you load your app using its private key there is no more work to do, all signing of requests is handled
automatically by the library.

### Get Client API Key Balance 

All clients start with a balance of zero credits available, which can be queried any time

#### Creating a New Client

```

let client: Get402::Client = app.create_client();


```

#### Getting Balance For An Existing Client

```

let client_identifier = env::var("GET402_API_CLIENT_IDENTIFIER").unwrap();

let client = app.get_client_from_identifier();

let balance: u64 = client.get_balance().await.unwrap();


```

### Charge Client API Key

When a client uses your API you should charge their API key which reduces their available balance of credits.

```

let client = app.get_client_from_identifier();

let mut params = HashMap::new();

params.insert("credits", 1);

let response = client.charge_credit(&params);

```

If their balance of credits goes to zero you will receive an error including a PaymentRequired request with details
on purchasing additional credits. If you do not want to receive an error here always check the balance first.

```

match client.charge_credit(&params).await {

    Err(Get402::APIError::InsufficientFunds(payment_request)) => {

        println!("Insufficient Funds!");
        // payment_request  includes outputs array, paymentUrl, and memo
    }

    Ok() => {

      println!("Sufficient Credit For Call");

    }

    other => {
       println!("Unexpected Error Occurred!");
    }
}


```


### Add Funds To Client API Key

#### Getting a Payment Request To Buy More Credits

To purchase additional credits simply request a new payment template for any number of credits. You will receive a
standard payment request which wallets know how to fulfill.

```

let payment_request: PaymentRequired = client.request_buy_credits(10).await.unwrap();

```


#### Using Client Key To Purchase More Credits Directly

Since client API keys are actually public/private key pairs capable of holding funds directly, this library provides
a utility for purchasing new credits using the client private key directly. First you must load your client funds
by sending satoshis to the client identifier address. Once funds arrive they will be available for purchasing credits.

```


```

Once payment is sent your client API key will immediately be credited with additional credits


## Development & Testing

```
cargo build
```

To run the tests you must set `GET402_PRIVATE_KEY` environment variable either in the shell or a `.env` file

```
cargo test
```

