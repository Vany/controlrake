// ad astra per 𖫪

function ConnectWebsocket(handler) {
    let addr = "ws://" + location.host + "/" + handler
    let p = new Promise(function (resolve, reject) {
        let server = new WebSocket(addr);
        server.onopen = () => resolve(server);
        server.onerror = reject;
        server.onmessage = onWSMessage;
        server.SendBuffer = [];
    })

    p.then((server) => {
        WS = server;
        console.log("ws connected to " + handler)
        for (let msg in WS.SendBuffer) {
            WS.send(msg)
        }
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

function Send(obj, msg) {
    let pa  = obj.parentNode.closest(".widget");
    if (pa) {
        Send(pa, obj.id + "|" + msg)
    } else if (WS.readyState == WebSocket.OPEN) {
        WS.send(msg)
    } else {
        console.log("buffered: " + msg);
        WS.SendBuffer.push(msg)
    }
}

