import cli from "command-line-args";
import { Chat } from "./set-chat";
import { createRoom } from "./room/set-room";
import uWS from "uWebSockets.js";

const args = cli([{
    name: "port",
    alias: "p",
    type: Number,
    defaultValue: 42067,
}]);

const chat = new Chat(createRoom);

uWS.App()
    .ws("/*", {
        open(ws) {
        },

        message(ws, message, isBinary) {
        },

        close(ws, code, message) {
        }
    }).listen(args.port, (listenSocket) => {
        if (listenSocket) {
            console.log(`Listening to port ${args.port}`);
        } else {
            console.log("failed to listen");
            process.exit(1);
        }
    });

