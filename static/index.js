// ad astra per 𖫪

function ConnectWebsocket() {
    let p = new Promise(function (resolve, reject) {
        let server = new WebSocket("ws://" + location.host + "/ws");
        server.onopen = () => resolve(server);
        server.onerror = reject;
        server.onmessage = onWSMessage;
    })

    p.then((server) => {
        WS = server;
        console.log("connected")
        server.onclose = () => {
            if (WS.readyState != WebSocket.CONNECTING) setTimeout(ConnectWebsocket, 1000);
        };
    })
    .catch((err, ev) => {
        console.error(err);
        if (WS.readyState != WebSocket.CONNECTING) setTimeout(ConnectWebsocket, 1000);
    })
    ;
}


ConnectWebsocket()

fetch("/widgets/")
    .then((response) => response.text())
    .then((text) => {
        document.getElementById("content").innerHTML = text;
        // TODO use css selector here
        for (let w of document.getElementsByClassName("widget")) {
            for (let s of w.getElementsByTagName("script")) {
                eval(s.innerText);
            }
        }
    })
;


//var WS = new WebSocket
function Send(obj, msg) {
    let w = obj.closest(".widget")
    let name = w.id
    console.log("WS>", name, msg)
    if (WS.readyState == WebSocket.OPEN)
        WS.send(name + "|" +  msg)
    else
        console.log("Try to send to not open websocket");
}

function onWSMessage(ev) {
    const [name, data] = ev.data.split("|", 2);
    let w = document.getElementById(name);
    let f = w.onWSEvent;
    if (f != null) f(data)
    else console.log(name, " have no ws event listener");
}




