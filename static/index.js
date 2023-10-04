// ad astra per 𖫪

var WS = new WebSocket(null);

function ConnectWebsocket(handler) {
    let addr = "ws://" + location.host + "/" + handler
    let sb =  WS ? WS.SendBuffer : [];
    WS = new WebSocket(addr);
    WS.onmessage = onWSMessage;
    WS.SendBuffer = sb;

    WS.onopen = () => {
        console.log("ws connected to " + handler)
        for (let msg in WS.SendBuffer) {
            WS.send(msg)
        }
        WS.onclose = () => {
            if (WS.readyState != WebSocket.CONNECTING) setTimeout(() => ConnectWebsocket(handler), 1000);
        };
    };

    WS.onerror = () => {
        console.error(err);
        if (WS.readyState != WebSocket.CONNECTING) setTimeout(() => ConnectWebsocket(handler), 1000);
    };
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

