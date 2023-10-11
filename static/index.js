// ad astra per 𖫪

var WS = {};


function ConnectWebsocket(handler) {
    let addr = "ws://" + location.host + "/" + handler;
    WS = new WebSocket(addr);
    console.log("Connecting: " + handler)
    WS.onmessage = onWSMessage;

    WS.onopen = () => {
        console.log("Connected to " + handler)
        WS.onclose = () => {
            setTimeout(() => ConnectWebsocket(handler), 1000);
        };
        WS.onerror = null;
    };

    WS.onerror = (ev) => {
        setTimeout(() => ConnectWebsocket(handler), 1000);
    };
}

function Send(obj, msg) {
    let pa  = obj.parentNode.closest(".widget");
    if (pa) {
        Send(pa, obj.id + "|" + msg)
    } else if (WS.readyState == WebSocket.OPEN) {
        WS.send(msg)
    } else {
        console.log("lost: " + msg);
    }
}

