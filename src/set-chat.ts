import WebSocket from "ws";
import { IRoom } from "./types";

export class Chat {
    private rooms: Map<string, IRoom>;

    constructor(private createRoom: (name: string) => IRoom) {
        this.rooms = new Map();
    }

    add(user: WebSocket) {
        user.on("message", (msg) => {
            const message = typeof msg === "object" ? msg.toString() : msg;
            const [command, ...rest] = message.split(" ");
            if (command === "join") {
                this.join(user, rest[0]);
            } else if (command === "msg") {
                this.msg(user, rest[0], rest.slice(1).join(" "));
            } else if (command === "leave") {
                this.leave(user, rest[0]);
            }
        });

        user.on("error", (error: Error) => {
            console.error(error);
        });

        user.on("close", () => {
            this.rooms.forEach((room) => {
                room.remove(user);
            });
        });
    }

    private join(user: WebSocket, roomName: string): void {
        let room = this.findRoom(roomName);

        if (!room) {
            room = this.createRoom(roomName);
            this.rooms.set(roomName, room);
        }

        room.add(user);
    }

    private leave(user: WebSocket, roomName: string): void {
        let room = this.findRoom(roomName);
        if (!room) {
            user.send("you are a roomless jerk");
            return;
        }

        room.remove(user);
    }

    private msg(user: WebSocket, roomName: string | undefined, message: string): void {
        if (roomName === undefined || message.length === 0) {
            user.send("you are a jerk");
            return;
        }

        let room = this.findRoom(roomName);
        if (!room) {
            user.send("you are a roomless jerk");
            return;
        }

        room.push(user, message);
    }

    private findRoom(roomName: string): IRoom | undefined {
        return this.rooms.get(roomName);
    }
}


