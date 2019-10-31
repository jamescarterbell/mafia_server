from Client import Bot
from random import randint


class DQNAgent(Bot):

    def action(self, action_info: dict) -> list:
        num = randint(0, self.num_players)
        output = list()
        for i in range(0, self.num_players):
            output.append(1 if num == i else 0)
        return output
