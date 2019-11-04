from Client import Bot, Reward
from random import randint
from threading import Lock
import numpy as np
import time
import queue


class QReward(Reward):

    # Store's future reward modifier
    def __init__(self, gamma: float):
        self.gamma = gamma

    # Calculate the reward from the given action,
    # and the max reward of the following state
    '''
    (state_info, action_taken)
    '''

    def get_reward(self, state: list, hidden_info, future_reward: list, num_players: int, res=None, num=0) -> list():
        maxi = future_reward[0]
        for q in future_reward:
            q *= self.gamma
            if q > maxi:
                maxi = q
        reward = list()

        # Calculate the reward value for each vote
        for i in range(0, num_players):
            reward_calc = -1 if state[5 + i * (num_players + 1)] == 0 else 0
            reward_calc += 1 if state[1] == 2 and hidden_info[i][0] < 2 else -1
            reward_calc += 1 if state[1] != 2 and hidden_info[i][0] == 2 else 0
            if num in res:
                reward_calc += 1 if res[num][0] == i else 0
            maxi

            reward.append(reward_calc)
        return np.array(reward).reshape(1, -1)


class DQNAgent(Bot):

    def __init__(self, times: int, reward: Reward, model):
        super().__init__(times)
        self.reward = reward
        self.model = model
        self.game = 0.0
        self.survived = 0.0

    # Take in the action info, plug into NN to get action,
    # Save action input, and output for future reward calculations.
    '''
    Action info is as follows:
    {'Player': int,
     'Role': int,
     'Status': int,
     'Phase': int, DQNAgent(10, reward, model=model)
     'Day': 0,
     0: (status_int, list of length num_players of this players last guesses),
     ...
     num_players: (status_int, list of length num_players of this players last guesses),
     '''

    def start(self, id: str, role: str, status: str, num_players: int):
        super().start(id, role, status, num_players)
        self.state_action_pairs = list()
        self.known = {}
        self.game += 1
        self.alive = True
        self.result = {}

    def action(self, action_info: dict) -> list:
        model_input = list()
        model_input.append(int(action_info['Player']))
        model_input.append(action_info['Role'])
        model_input.append(action_info['Status'])
        model_input.append(action_info['Phase'])
        model_input.append(action_info['Day'])
        for i in range(0, self.num_players):
            model_input.append(action_info[i][0])
            if i in self.known:
                model_input.append(self.known[i])
            else:
                model_input.append(-1)
            if action_info[i][1]:
                for j in range(0, self.num_players):
                    model_input.append(action_info[i][1][j])
            else:
                model_input.extend([0 for k in range(0, self.num_players)])

        model_input = np.array(model_input).reshape(1, -1)
        output = self.model.predict(model_input)
        self.state_action_pairs.append((model_input[0], output[0]))
        return output[0]

    # Take in ending info, use this for training purposes
    # Includes final player statuses, as well as all hidden roles
    '''
    {0: (role_int, status_int),
    ...
    num_players: (role_int, status_int)}
    '''

    def end(self, ending_info: dict):
        sarf = list()
        if ending_info[self.id][1] == 1:
            self.survived += 1
        print(self.survived / self.game)
        for i in range(0, len(self.state_action_pairs)-1):
            sarf.append((self.state_action_pairs[i][0],
                         self.reward.get_reward(
                        self.state_action_pairs[i][0], ending_info, self.state_action_pairs[i + 1][1], self.num_players, self.result, i)))
        self.model.train(sarf)

    # Take in info_info, use to store responses from votes
    # Should almost always be a dead status, mainly used
    # to learn confirmed roles of the dead.
    '''
    (player_int, role_int, status_int)
    '''

    def info(self, info_info: tuple):
        self.result[len(self.state_action_pairs) - 1] = info_info
        self.known[info_info[0]] = info_info[1]
        pass


class Trainer():

    def __init__(self, model):
        self.model = model
        self.in_use = Lock()

    def predict(self, input_data):
        self.in_use.acquire()
        output = self.model.predict(input_data)
        self.in_use.release()
        return output

    def train(self, info):
        info = np.array(info)
        self.in_use.acquire()
        try:
            input_data = list()
            output_data = list()
            for arr in info[:, 0]:
                input_data.append(arr.reshape(-1, 1))
            for arr in info[:, 1]:
                output_data.append(arr.reshape(-1, 1))
            input_data = np.array(input_data)[:, :, 0]
            output_data = np.array(output_data)[:, :, 0]
            self.model.fit(input_data,
                           output_data)
        except ValueError as e:
            print(e)
        self.in_use.release()
