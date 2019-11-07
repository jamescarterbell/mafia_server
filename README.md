# mafia_server
Mafia Game + DQN's to play it for a Machine Learning class project at UCF

Instructions to use:

Install rust nightly to run the server, then in the server directory
```
cargo run
```

To run the agents, go into the Agents directory and run RunAgents.py

If you want to change the model the DQN's use, change the model made in RunAgents.py.

If you want to use an entirely different agent type, import Bot from Client.py, and write a subclass that implements it's own version of start(), end(), info(), and action().  The inputs are given in DQN for guidance.  To write your own reward function, import Reward from Client.py and implement get_reward().

If you want to change where the bots connect to, pass the url to the bot constructor as ```game_url="www.website.com"```.

Future Work:

1.  Rework the server package to make writing other games much easier: includes further seperating of server logic and game logic.
2.  JSONize everything.
3.  Move from loops to events.
4.  Clean up code and add more comments.
