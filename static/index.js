// ad astra per 𖫪

function ConnectWebsocket(handler) {
    let addr = "ws://" + location.host + "/" + handler
    let p = new Promise(function (resolve, reject) {
        let server = new WebSocket(addr);
        server.onopen = () => resolve(server);
        server.onerror = reject;
        server.onmessage = onWSMessage;
    })

    p.then((server) => {
        WS = server;
        console.log("ws connected to " + handler)
        server.onclose = () => {
            if (WS.readyState != WebSocket.CONNECTING) setTimeout(() => ConnectWebsocket(handler), 1000);
        };
    })
    .catch((err, ev) => {
        console.error(err);
        if (WS.readyState != WebSocket.CONNECTING) setTimeout(() => ConnectWebsocket(handler), 1000);
    })
    ;
}

function FetchWidgets() {
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
}


//var WS = new WebSocket
function Send(obj, msg) {
    let w = obj.closest(".widget");
    let name = w.id;
    if (WS.readyState == WebSocket.OPEN)
        WS.send(name + "|" +  msg)
    else
        console.error("Try to send to not open websocket");
}

