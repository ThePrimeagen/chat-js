import ws from "ws";
import cli from "command-line-args";
import { Chat } from "./set-chat";
import { createRoom } from "./room/set-room";

const args = cli([{
    name: "port",
    alias: "p",
    type: Number,
    defaultValue: 42067,
}]);

const wss = new ws.Server({ port: args.port });
const chat = new Chat(createRoom);

wss.on("connection", (ws) => {
    chat.add(ws);
});

