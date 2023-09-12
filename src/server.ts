import ws from "ws";
import { Chat } from "./set-chat";
import { createRoom } from "./room/set-room";
import { initConfig } from "./cli";
import { initLogger } from "./logger";

const config = initConfig();
initLogger(config);
const wss = new ws.Server({ port: config.port });
const chat = new Chat(createRoom);

wss.on("connection", (ws) => {
    chat.add(ws);
});

wss.on("listening", () => {
    console.log("WE ARE A GO HOUSTONE");
});

