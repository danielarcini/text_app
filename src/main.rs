use rocket::fs::{relative,FileServer};
use rocket::{State, Shutdown};
use rocket::response::stream::{EventStream, Event};
use rocket::serde::{Serialize, Deserialize};
use rocket::tokio::sync::broadcast::{Sender, channel, error::RecvError};
use rocket::tokio::sync::broadcast::error::SendError;
use rocket::form::Form;
use rocket::tokio::select;

//#[] are macros of rocket

#[macro_use] extern crate rocket; //lets us use the rocket macro anywhere (globally)

#[derive(Debug, Clone, FromForm, Serialize, Deserialize)] //debug so struct can be printed out with debug format, clone to duplicate messages, take form data to message struct, serialize and deseralize will let the struct data structures to get serialized and deseralized
#[serde(crate = "rocket::serde")] //serde crate definied in rocket
struct Message {
    #[field(validate = len(..20))]// room name can only be 19 characters long
    pub room: String,
    #[field(validate = len(..21))] //username to be 20 characters long
    pub username: String,
    pub message: String,
}

#[post("/message", data = "<form>")]
fn post_mssg (form: Form<Message>, queue: &State<Sender<Message>>) {
    //send fails if there is no active subscribers.
    let _res:Result<usize, SendError<Message>> = queue.send(form.into_inner());
}

#[get("/events")] //get request to the events path
async fn events(queue: &State<Sender<Message>>, mut end: Shutdown) -> EventStream! []{ //server sent events are produced async
    let mut rx = queue.subscribe();

    EventStream! { //server can send data to clients. Similar to web clients but clients cant sent data back to server.
        loop {
            let msg = select! {
                msg = rx.recv () => match msg { //calls reciever on msg
                    Ok(msg) =>msg, //ok variant returns the message inside of it
                    Err(RecvError::Closed) => break, //no more senders so loop is broke
                    Err(RecvError::Lagged(_)) => continue, //reciever lagged to far behind and so we skip
                },
                _= &mut end => break, //waiting for shutdown future to resolve to break loop
            };
            yield Event::json (&msg); //if we break we yield to a new server 
        }
    }
}

#[launch]
fn rocket() -> _ {
    rocket:: build()
        .manage(channel:: <Message>(1024).0) //what type of message to send in channel. .0 lets us get the first element 1024 is the amount of messages we can hold
        .mount("/", routes![post_mssg, events])
        .mount("/", FileServer::from(relative!("static")))
}