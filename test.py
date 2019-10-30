import requests
import socket
import time
import asyncio
from multiprocessing import Pool
from random import randint


class Bot:

    def __init__(self, times: int):
        self.roles = {"Innocent": 0,
                      "Detective": 1,
                      "Mafia": 2, }
        self.statuses = {"Alive": 1,
                         "Dead": 0}
        self.times = times

    def run_bot_join(self):
        loop = asyncio.get_event_loop()
        loop.run_until_complete(self.run_bot())

    def run_bot(self):
        for i in range(0, self.times):
            port = requests.get("http://localhost:8000/new_connection")
            s = socket.create_connection(("localhost", int(port.text)))
            while(True):
                try:
                    # Read in the length of the message, then the message itself:
                    length = int.from_bytes(
                        s.recv(8), byteorder='big', signed=False)
                    message = s.recv(length).decode("utf-8")
                    if length == 0:
                        continue

                    # Split the message into its principal components, then decide which function to call.
                    message = message.replace(" ", "").split(",")
                    if message[0] == "Start":
                        message_broken = list()
                        for piece in message:
                            parts = piece.split(":")
                            message_broken.extend(parts)
                        self.start(
                            message_broken[2], message_broken[4], message_broken[6])

                    elif message[0] == "End":
                        self.end(self.create_ending_dict(message[1:]))
                    else:
                        num = randint(0, 8)
                        message = ""
                        for i in range(0, 8):
                            message += "1," if num == i else "0,"
                        message = message.encode("utf-8")
                        length = len(message).to_bytes(
                            8, byteorder='big', signed=False)

                        s.send(length)
                        s.send(message)
                except:
                    break

    # This will initialize some game logic that the player can keep track of
    def start(self, id: str, role: str, status: str):
        self.id = int(id)
        self.role = self.roles[role]
        self.status = self.statuses[status]
        print("Started")

    # This will do stuff with ending_info, use this for trianing purposes
    def end(self, ending_info: dict):
        print("Ending stuff")

    def create_ending_dict(self, message: list()):
        dictionary = {}
        last_player = 0
        for pair in message:
            broken_pair = pair.split(":")
            if broken_pair[0] == "Player":
                last_player = int(broken_pair[1])
            elif last_player not in dictionary:
                dictionary[last_player] = broken_pair[1]
            else:
                dictionary[last_player] = (
                    dictionary[last_player], broken_pair[1])
        return dictionary


pool = Pool(100)
bots = [Bot(5) for i in range(0, 100)]

for bot in bots:
    pool.apply_async(bot.run_bot_join)
pool.close()
pool.join()
