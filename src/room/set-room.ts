import WebSocket from "ws";
import { IRoom } from "../types";

export default class SetRoom {
    private users: Set<WebSocket>;

    constructor(public name: string) {
        this.users = new Set();
    }

    add(user: WebSocket) {
        this.users.add(user);
    }

    remove(user: WebSocket) {
        this.users.delete(user);
    }

    push(from: WebSocket, message: string) {
        for (const sock of this.users) {
            sock.send(`${from} says ${message}`);
        }
    }
}

export function createRoom(name: string): IRoom {
    return new SetRoom(name);
}

