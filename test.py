import requests
import socket
import time
import asyncio
from multiprocessing import Pool
from random import randint


async def openConnections():
    port = requests.get("http://localhost:8000/new_connection")
    print(int(port.text))
    s = socket.create_connection(("localhost", int(port.text)))
    i = False
    while(True):
        length = int.from_bytes(s.recv(8), byteorder='big', signed=False)
        message = s.recv(length).decode("utf-8")
        if not i or length == 0:
            i = True
            continue
        print(message)
        num = randint(0, 8)
        message = ""
        for i in range(0, 8):
            message += "1," if num == i else "0,"
        message = message.encode("utf-8")
        length = len(message).to_bytes(8, byteorder='big', signed=False)
        s.send(length)
        s.send(message)


def bot(i):
    loop = asyncio.get_event_loop()
    loop.run_until_complete(openConnections())


pool = Pool(8)
pool.map(bot, [i for i in range(0, 8)])
