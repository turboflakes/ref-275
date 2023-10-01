use anyhow::anyhow;
use futures::FutureExt;

use subxt::{OnlineClient, PolkadotConfig};

use subxt::ext::codec::{Decode, Encode};
use subxt::tx::SubmittableExtrinsic;
use subxt::tx::TxPayload;
use subxt::utils::{AccountId32, MultiSignature};

use crate::services::{
    extension_signature_for_extrinsic, get_accounts, node_runtime,
    node_runtime::runtime_types::pallet_conviction_voting::vote::{AccountVote, Vote},
    subscribe_to_finalized_blocks, Account,
};
use strum::IntoEnumIterator;
use strum_macros::EnumIter;
use web_sys::HtmlInputElement;
use yew::prelude::*;

pub struct VoteComponent {
    message: String,
    conviction: Conviction,
    balance: u128,
    remark_call_bytes: Vec<u8>,
    vote_call_bytes: Vec<u8>,
    online_client: Option<OnlineClient<PolkadotConfig>>,
    stage: SigningStage,
    finalized_blocks: Vec<AttrValue>,
}

impl VoteComponent {
    /// # Panics
    /// panics if self.online_client is None.
    fn set_message(&mut self, message: String) {
        let remark_call = node_runtime::tx()
            .system()
            .remark(message.as_bytes().to_vec());
        let online_client = self.online_client.as_ref().unwrap();
        let remark_call_bytes = remark_call
            .encode_call_data(&online_client.metadata())
            .unwrap();
        self.remark_call_bytes = remark_call_bytes;
        self.message = message;
    }

    fn set_vote(&mut self, balance: u128, conviction: Conviction) {
        let vote_call = node_runtime::tx().conviction_voting().vote(
            275,
            AccountVote::Standard {
                vote: Vote(conviction.to_value()),
                balance: balance * 1000000000000,
            },
        );
        let online_client = self.online_client.as_ref().unwrap();
        let vote_call_bytes = vote_call
            .encode_call_data(&online_client.metadata())
            .unwrap();
        self.vote_call_bytes = vote_call_bytes;
        self.balance = balance;
        self.conviction = conviction;
    }

    fn is_selected(&self, conviction: Conviction) -> String {
        if self.conviction == conviction {
            return "selected".to_string();
        }
        "".to_string()
    }
}

pub enum SigningStage {
    Error(String),
    CreatingOnlineClient,
    EnterMessage,
    EnterBalance,
    RequestingAccounts,
    SelectAccount(Vec<Account>),
    Signing(Account),
    SigningSuccess {
        signer_account: Account,
        signature: MultiSignature,
        signed_extrinsic_hex: String,
        submitting_stage: SubmittingStage,
    },
}

pub enum SubmittingStage {
    Initial {
        signed_extrinsic: SubmittableExtrinsic<PolkadotConfig, OnlineClient<PolkadotConfig>>,
    },
    Submitting,
    Success {
        remark_event: node_runtime::system::events::ExtrinsicSuccess,
    },
    Error(anyhow::Error),
}

pub enum Message {
    Error(anyhow::Error),
    OnlineClientCreated(OnlineClient<PolkadotConfig>),
    ChangeMessage(String),
    ChangeBalance(String),
    ChangeConviction(Conviction),
    RequestAccounts,
    ReceivedAccounts(Vec<Account>),
    /// usize represents account index in Vec<Account>
    SignWithAccount(usize),
    ReceivedSignature(
        MultiSignature,
        SubmittableExtrinsic<PolkadotConfig, OnlineClient<PolkadotConfig>>,
    ),
    SubmitSigned,
    ExtrinsicFinalized {
        remark_event: node_runtime::system::events::ExtrinsicSuccess,
    },
    ExtrinsicFailed(anyhow::Error),
    SubscribeFinalizedBlock,
    PushFinalizedBlock(AttrValue),
}

const LOCK1X: u8 = 129;
const LOCK2X: u8 = 130;
const LOCK3X: u8 = 131;
const LOCK4X: u8 = 132;
const LOCK5X: u8 = 133;
const LOCK6X: u8 = 134;

#[derive(Clone, PartialEq, EnumIter)]
pub enum Conviction {
    Lock1X,
    Lock2X,
    Lock3X,
    Lock4X,
    Lock5X,
    Lock6X,
}

impl Conviction {
    pub fn to_value(&self) -> u8 {
        match &self {
            Self::Lock1X => LOCK1X,
            Self::Lock2X => LOCK2X,
            Self::Lock3X => LOCK3X,
            Self::Lock4X => LOCK4X,
            Self::Lock5X => LOCK5X,
            Self::Lock6X => LOCK6X,
        }
    }
}

impl std::fmt::Display for Conviction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Lock1X => write!(f, "1x"),
            Self::Lock2X => write!(f, "2x"),
            Self::Lock3X => write!(f, "3x"),
            Self::Lock4X => write!(f, "4x"),
            Self::Lock5X => write!(f, "5x"),
            Self::Lock6X => write!(f, "6x"),
        }
    }
}

impl Component for VoteComponent {
    type Message = Message;

    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        ctx.link().send_future(OnlineClient::<PolkadotConfig>::from_url("wss://rpc.ibp.network/kusama").map(|res| {
            match res {
                Ok(online_client) => Message::OnlineClientCreated(online_client),
                Err(err) => Message::Error(anyhow!("Online Client could not be created. Make sure you have a local node running:\n{err}")),
            }
        }));
        VoteComponent {
            message: "".to_string(),
            conviction: Conviction::Lock1X,
            balance: 100,
            stage: SigningStage::CreatingOnlineClient,
            online_client: None,
            remark_call_bytes: vec![],
            vote_call_bytes: vec![],
            finalized_blocks: vec![],
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Message::OnlineClientCreated(online_client) => {
                self.online_client = Some(online_client);
                // self.stage = SigningStage::EnterMessage;
                // self.set_message("Hello".into());
                self.stage = SigningStage::EnterBalance;
                self.set_vote(1, Conviction::Lock1X);
            }
            Message::ChangeMessage(message) => {
                self.set_message(message);
            }
            Message::ChangeBalance(balance) => {
                let value = balance.parse::<u128>().unwrap_or(100);
                self.set_vote(value, self.conviction.clone());
            }
            Message::ChangeConviction(conviction) => {
                self.set_vote(self.balance, conviction);
            }
            Message::RequestAccounts => {
                self.stage = SigningStage::RequestingAccounts;
                ctx.link().send_future(get_accounts().map(
                    |accounts_or_err| match accounts_or_err {
                        Ok(accounts) => Message::ReceivedAccounts(accounts),
                        Err(err) => Message::Error(err),
                    },
                ));
            }
            Message::ReceivedAccounts(accounts) => {
                self.stage = SigningStage::SelectAccount(accounts);
            }
            Message::Error(err) => self.stage = SigningStage::Error(err.to_string()),
            Message::SignWithAccount(i) => {
                if let SigningStage::SelectAccount(accounts) = &self.stage {
                    let account = accounts.get(i).unwrap();
                    let account_address = account.address.clone();
                    let account_source = account.source.clone();
                    let account_id: AccountId32 = account_address.parse().unwrap();

                    self.stage = SigningStage::Signing(account.clone());

                    let vote_call = node_runtime::tx().conviction_voting().vote(
                        275,
                        AccountVote::Standard {
                            vote: Vote(self.conviction.to_value()),
                            balance: self.balance * 1000000000000,
                        },
                    );

                    let api = self.online_client.as_ref().unwrap().clone();

                    ctx.link().send_future(async move {
                        let Ok(account_nonce) = api.tx().account_nonce(&account_id).await else {
                            return Message::Error(anyhow!("Fetching account nonce failed"));
                        };

                        let Ok(call_data) = api.tx().call_data(&vote_call) else {
                            return Message::Error(anyhow!("could not encode call data"));
                        };

                        let Ok(signature) = extension_signature_for_extrinsic(
                            &call_data,
                            &api,
                            account_nonce,
                            account_source,
                            account_address,
                        )
                        .await
                        else {
                            return Message::Error(anyhow!("Signing via extension failed"));
                        };

                        let Ok(multi_signature) = MultiSignature::decode(&mut &signature[..])
                        else {
                            return Message::Error(anyhow!("MultiSignature Decoding"));
                        };

                        let Ok(partial_signed) = api.tx().create_partial_signed_with_nonce(
                            &vote_call,
                            account_nonce,
                            Default::default(),
                        ) else {
                            return Message::Error(anyhow!("PartialExtrinsic creation failed"));
                        };

                        // Apply the signature
                        let signed_extrinsic = partial_signed
                            .sign_with_address_and_signature(&account_id.into(), &multi_signature);

                        // check the TX validity (to debug in the js console if the extrinsic would work)
                        // let dry_res = signed_extrinsic.validate().await;
                        // web_sys::console::log_1(&format!("Validation Result: {:?}", dry_res).into());

                        // return the signature and signed extrinsic
                        Message::ReceivedSignature(multi_signature, signed_extrinsic)
                    });
                }
            }
            Message::ReceivedSignature(signature, signed_extrinsic) => {
                if let SigningStage::Signing(account) = &self.stage {
                    let signed_extrinsic_hex =
                        format!("0x{}", hex::encode(signed_extrinsic.encoded()));
                    self.stage = SigningStage::SigningSuccess {
                        signer_account: account.clone(),
                        signature,
                        signed_extrinsic_hex,
                        submitting_stage: SubmittingStage::Initial { signed_extrinsic },
                    }
                }
            }
            Message::SubmitSigned => {
                if let SigningStage::SigningSuccess {
                    submitting_stage: submitting_stage @ SubmittingStage::Initial { .. },
                    ..
                } = &mut self.stage
                {
                    let SubmittingStage::Initial { signed_extrinsic } =
                        std::mem::replace(submitting_stage, SubmittingStage::Submitting)
                    else {
                        panic!("unreachable")
                    };

                    ctx.link().send_future(async move {
                        match submit_wait_finalized_and_get_extrinsic_success_event(
                            signed_extrinsic,
                        )
                        .await
                        {
                            Ok(remark_event) => Message::ExtrinsicFinalized { remark_event },
                            Err(err) => Message::ExtrinsicFailed(err),
                        }
                    });
                }
            }
            Message::ExtrinsicFinalized { remark_event } => {
                if let SigningStage::SigningSuccess {
                    submitting_stage, ..
                } = &mut self.stage
                {
                    *submitting_stage = SubmittingStage::Success { remark_event }
                }
            }
            Message::ExtrinsicFailed(err) => {
                if let SigningStage::SigningSuccess {
                    submitting_stage, ..
                } = &mut self.stage
                {
                    *submitting_stage = SubmittingStage::Error(err)
                }
            }
            Message::PushFinalizedBlock(block_attr) => {
                // newer lines go to the top
                self.finalized_blocks.insert(0, block_attr);
                // remove older block number
                if self.finalized_blocks.len() > 1 {
                    self.finalized_blocks.truncate(1);
                }
            }
            Message::SubscribeFinalizedBlock => {
                let cb: Callback<AttrValue> = ctx.link().callback(Message::PushFinalizedBlock);
                ctx.link()
                    .send_future(subscribe_to_finalized_blocks(cb).map(|result| {
                        let err = result.unwrap_err();
                        Message::Error(err.into())
                    }));
            }
        };
        true
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let message_as_hex_html = || {
            html!(
                <div class="mb">
                    <b>{"Hex representation of \"remark\" call in \"System\" pallet:"}</b> <br/>
                    {format!("0x{}", hex::encode(&self.remark_call_bytes))}
                </div>
            )
        };

        let _message_html: Html = match &self.stage {
            SigningStage::Error(_)
            | SigningStage::EnterMessage
            | SigningStage::CreatingOnlineClient => html!(<></>),
            _ => {
                let _remark_call = node_runtime::tx()
                    .system()
                    .remark(self.message.as_bytes().to_vec());

                html!(
                    <div>
                        <div class="mb">
                            <b>{"Message: "}</b> <br/>
                            {&self.message}
                        </div>
                        {message_as_hex_html()}
                    </div>
                )
            }
        };

        let vote_as_hex_html = || {
            let encoded_call = format!("0x{}", hex::encode(&self.vote_call_bytes));
            let url = format!("https://polkadot.js.org/apps/?rpc=wss://rpc.ibp.network/kusama#/extrinsics/decode/{}", encoded_call);
            html!(
                <div class="mb">
                    <b>{"Encoded call data:"}</b> <br/>
                    <a href={url} target="_blank">{encoded_call}</a>
                </div>
            )
        };

        let subscribe_finalized =
            ctx.link().callback(|_| Message::SubscribeFinalizedBlock);

        let _finalized_block_html: Html = {
            html!(
                <div>
                    if self.finalized_blocks.is_empty(){
                        <button onclick={subscribe_finalized} >{"subscribe finalized blocks"}</button>
                    }
                    { for self.finalized_blocks.iter().map(|line| html! {<p> {line} </p>}) }
                </div>
            )
        };

        let vote_html: Html = match &self.stage {
            SigningStage::Error(_)
            | SigningStage::EnterBalance
            | SigningStage::CreatingOnlineClient => html!(<></>),
            _ => {
                let _vote_call = node_runtime::tx().conviction_voting().vote(
                    275,
                    AccountVote::Standard {
                        vote: Vote(129),
                        balance: 10000000000000,
                    },
                );
                html!(
                    <div>
                        {vote_as_hex_html()}
                    </div>
                )
            }
        };

        let signer_account_html: Html = match &self.stage {
            SigningStage::Signing(signer_account)
            | SigningStage::SigningSuccess { signer_account, .. } => {
                html!(
                    <div class="mb">
                            <b>{"Account used for signing: "}</b> <br/>
                            {"Extension: "}{&signer_account.source} <br/>
                            {"Name: "}{&signer_account.name} <br/>
                            {"Address: "}{&signer_account.address} <br/>
                    </div>
                )
            }
            _ => html!(<></>),
        };

        let stage_html: Html = match &self.stage {
            SigningStage::Error(error_message) => {
                html!(<div class="error"> {"Error: "} {error_message} </div>)
            }
            SigningStage::CreatingOnlineClient => {
                html!(
                    <div>
                        <b>{"Creating Online Client..."}</b>
                    </div>
                )
            }
            SigningStage::EnterMessage => {
                let get_accounts_click = ctx.link().callback(|_| Message::RequestAccounts);
                let on_input = ctx.link().callback(move |event: InputEvent| {
                    let input_element = event.target_dyn_into::<HtmlInputElement>().unwrap();
                    let value = input_element.value();
                    Message::ChangeMessage(value)
                });

                html!(
                    <>
                        <div class="mb"><b>{"Enter a message for the \"remark\" call in the \"System\" pallet:"}</b></div>
                        <input oninput={on_input} class="mb" value={AttrValue::from(self.message.clone())}/>
                        {message_as_hex_html()}
                        <button onclick={get_accounts_click}> {"=> Select an Account for Signing"} </button>
                    </>
                )
            }
            SigningStage::EnterBalance => {
                let get_accounts_click = ctx.link().callback(|_| Message::RequestAccounts);
                let on_input_balance = ctx.link().callback(move |event: InputEvent| {
                    let input_element = event.target_dyn_into::<HtmlInputElement>().unwrap();
                    let value = input_element.value();
                    Message::ChangeBalance(value)
                });

                html!(
                    <>
                        <div class="mb"><b>{"Enter AYE vote value (KSM):"}</b></div>
                        <input oninput={on_input_balance} class="mb" value={AttrValue::from(self.balance.to_string())}/>
                        <div><b>{"Conviction:"}</b></div>
                        <div class="mb" style="display: flex;">
                            { for Conviction::iter().map(|conviction| {
                                    let label = format!("Lock {}", conviction.clone());
                                    let class = self.is_selected(conviction.clone());
                                    let on_click_conviction = ctx.link().callback(move |_| Message::ChangeConviction(conviction.clone()));
                                    html! {
                                        <button class={class} onclick={on_click_conviction}>
                                            {label}
                                        </button>
                                    }
                                })
                            }
                        </div>
                        {vote_as_hex_html()}
                        <button onclick={get_accounts_click}> {"=> Select an Account for Signing"} </button>
                    </>
                )
            }
            SigningStage::RequestingAccounts => {
                html!(<div>{"Querying extensions for accounts..."}</div>)
            }
            SigningStage::SelectAccount(accounts) => {
                if accounts.is_empty() {
                    html!(<div>{"No Web3 extension accounts found. Install Talisman or the Polkadot.js extension and add an account."}</div>)
                } else {
                    html!(
                        <>
                            <div class="mb"><b>{"Select an account you want to use for signing:"}</b></div>
                            <div class="accounts">
                                { for accounts.iter().enumerate().map(|(i, account)| {
                                    let sign_with_account = ctx.link().callback(move |_| Message::SignWithAccount(i));
                                    html! {
                                        <button onclick={sign_with_account}>
                                            {&account.source} {" | "} {&account.name}<br/>
                                            <small>{&account.address}</small>
                                        </button>
                                    }
                                }) }
                            </div>
                        </>
                    )
                }
            }
            SigningStage::Signing(_) => {
                html!(<div>{"Singing message with browser extension..."}</div>)
            }
            SigningStage::SigningSuccess {
                signature,
                signed_extrinsic_hex,
                submitting_stage,
                ..
            } => {
                let submitting_stage_html = match submitting_stage {
                    SubmittingStage::Initial { .. } => {
                        let submit_extrinsic_click =
                            ctx.link().callback(move |_| Message::SubmitSigned);
                        html!(<button onclick={submit_extrinsic_click}> {"=> Submit the signed extrinsic"} </button>)
                    }
                    SubmittingStage::Submitting => {
                        html!(<div class="loading"><b>{"Submitting Extrinsic... (please wait a few seconds)"}</b></div>)
                    }
                    SubmittingStage::Success { remark_event } => {
                        html!(<div style="overflow-wrap: break-word;"> <b>{"Successfully submitted Extrinsic. Event:"}</b> <br/> {format!("{:?}", remark_event)} </div>)
                    }
                    SubmittingStage::Error(err) => {
                        html!(<div class="error"> {"Error: "} {err.to_string()} </div>)
                    }
                };

                html!(
                    <>
                        <div style="overflow-wrap: break-word;" class="mb">
                            <b>{"Received signature: "}</b><br/>
                            {hex::encode(signature.encode())}
                        </div>
                        <div style="overflow-wrap: break-word;" class="mb">
                            <b>{"Hex representation of signed extrinsic: "}</b> <br/>
                            {signed_extrinsic_hex}
                        </div>
                        {submitting_stage_html}
                    </>
                )
            }
        };

        html! {
            <div>
                <div class="header">
                    <span class="kusama-logo">
                        <img src="https://raw.githubusercontent.com/turboflakes/ref-275/main/assets/kusama_icon_shadow.svg" alt="kusama logo" />
                    </span>
                    <h1>{"ref. "}<a class="header-link" href="https://kusama.subsquare.io/referenda/275" target="_blank">{"#275"}</a></h1>
                </div>
                <h4>
                    {format!("Vote AYE with {} KSM and {} conviction", &self.balance, &self.conviction)}
                </h4>
                // {finalized_block_html}
                {vote_html}
                {signer_account_html}
                {stage_html}
                <div class="footer">
                    <a class="github-logo" href="https://github.com/turboflakes/ref-275" target="_blank">
                        <img src="https://raw.githubusercontent.com/turboflakes/ref-275/main/assets/github.svg" alt="github logo" />
                    </a>
                    <div class="powered">{"© 2023 Powered by TurboFlakes"}</div>
                </div>
            </div>
        }
    }
}

async fn submit_wait_finalized_and_get_extrinsic_success_event(
    extrinsic: SubmittableExtrinsic<PolkadotConfig, OnlineClient<PolkadotConfig>>,
) -> Result<node_runtime::system::events::ExtrinsicSuccess, anyhow::Error> {
    let events = extrinsic
        .submit_and_watch()
        .await?
        .wait_for_finalized_success()
        .await?;

    let events_str = format!("{:?}", &events);
    web_sys::console::log_1(&events_str.into());
    for event in events.find::<node_runtime::system::events::ExtrinsicSuccess>() {
        web_sys::console::log_1(&format!("{:?}", event).into());
    }

    let success = events.find_first::<node_runtime::system::events::ExtrinsicSuccess>()?;
    success.ok_or(anyhow!("ExtrinsicSuccess not found in events"))
}
