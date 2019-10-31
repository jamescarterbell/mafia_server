import requests
import socket
import asyncio
from random import randint
#from DQN import DQNAgent


class Bot:

    def __init__(self, times: int):
        self.roles = {"Innocent":  0,
                      "Detective": 1,
                      "Mafia":     2}

        self.statuses = {"Alive": 1,
                         "Dead":  0}

        self.phases = {"Detect":    0,
                       "PreVote":   1,
                       "Vote":      2,
                       "PreKill":   3,
                       "Kill":      4}
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
                            message_broken[2], message_broken[4], message_broken[6], message_broken[7])

                    elif message[0] == "End":
                        self.end(self.create_ending_dict(message[1:]))
                        break
                    else:
                        output = self.action(self.create_action_dict(message))

                        message = ""
                        for num in output:
                            message += str(num) + ","
                        message = message.encode("utf-8")
                        length = len(message).to_bytes(
                            8, byteorder='big', signed=False)

                        s.send(length)
                        s.send(message)
                except:
                    break

    # This will initialize some game logic that the player can keep track of
    def start(self, id: str, role: str, status: str, num_players: int):
        self.id = int(id)
        self.role = self.roles[role]
        self.status = self.statuses[status]
        self.num_players = int(num_players)
        print("Started")

    # This will do stuff with ending_info, use this for trianing purposes
    def end(self, ending_info: dict):
        print("Ended")

    def create_ending_dict(self, message: list) -> dict:
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

    # This will do stuff during the game, it must return a list
    # of the given length.
    def action(self, action_info: dict) -> list:
        num = randint(0, self.num_players)
        output = list()
        for i in range(0, self.num_players):
            output.append(1 if num == i else 0)
        return output

    def create_action_dict(self, message: list) -> dict:
        dictionary = {}
        last_player = -1
        first_status = False
        for pair in message:
            broken_pair = pair.split(":")
            if broken_pair[0] == "Status" and first_status:
                last_player += 1
                dictionary[last_player] = (broken_pair[1], list())
            elif len(broken_pair) == 1:
                dictionary[last_player][1].append(int(broken_pair[0]))
            else:
                if broken_pair[0] == "Status":
                    first_status = True
                    dictionary[broken_pair[0]] = self.statuses[broken_pair[1]]

                elif broken_pair[0] == "Day":
                    dictionary[broken_pair[0]] = int(broken_pair[1])

                elif broken_pair[0] == "Phase":
                    dictionary[broken_pair[0]] = self.phases[broken_pair[1]]
                else:
                    dictionary[broken_pair[0]] = broken_pair[1]
        print(dictionary)
        return dictionary
