from Client import Bot, Reward
from random import randint
from threading import Lock
import numpy as np
import time
import queue
import random


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
            if q > maxi:
                maxi = q
        reward = list()

        can_win, hunt = self.check_win(
            state, num_players, hidden_info, state[1])

        # Calculate the reward value for each vote
        for i in range(0, num_players):
            reward_calc = 0
            if state[5 + i * (num_players + 1)] == 0:
                reward_calc -= 1
                reward.append(reward_calc)
                continue
            reward_calc += 1 if state[1] == 2 and hidden_info[i][0] < 2 else -1
            reward_calc += 2 if state[1] != 2 and hidden_info[i][0] == 2 else -1

            if can_win:
                if i in hunt:
                    reward_calc += 5
            else:
                reward_calc += self.gamma * maxi

            reward.append(reward_calc)
        return np.array(reward).reshape(1, -1)

    def check_win(self, state: list, num_players, hidden_info, player_role):
        innocent_left = 0
        innocent_members = list()
        mafia_left = 0
        mafia_members = list()
        for i in range(0, num_players):
            if state[5 + i * (num_players + 1)] == 1:
                if hidden_info[i][0] == 2:
                    mafia_left += 1
                    mafia_members.append(i)
                else:
                    innocent_left += 1
                    innocent_members.append(i)

        if player_role < 2:
            if mafia_left == 1:
                return (True, mafia_members)
        else:
            if innocent_left - 1 == mafia_left:
                return (True, innocent_members)
        return (False, None)


class DQNAgent(Bot):

    def __init__(self, times: int, reward: Reward, model, epsilon, document=False):
        super().__init__(times)
        self.reward = reward
        self.model = model
        self.game = 0.0
        self.survived = 0.0
        self.doc = document
        self.eps = epsilon
        if self.doc:
            self.mafia_wins = 0
            self.detective_kills = 0
            self.avg_last_day = 0

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
        self.last_day = 0

    def action(self, action_info: dict) -> list:
        r = random.random()
        model_input = list()
        model_input.append(int(action_info['Player']))
        model_input.append(action_info['Role'])
        model_input.append(action_info['Status'])
        model_input.append(action_info['Phase'])
        model_input.append(action_info['Day'])
        self.last_day = action_info['Day']
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

        into_model = np.array(model_input).reshape(1, -1)

        if r < self.eps:
            try:
                output = self.model.predict(into_model)
            except:
                num = randint(0, self.num_players)
                output = list()
                for i in range(0, self.num_players):
                    output.append(1 if num == i else -1)
                output = [output]
        else:
            num = randint(0, self.num_players)
            output = list()
            for i in range(0, self.num_players):
                output.append(1 if num == i else -1)
            output = [output]

        self.state_action_pairs.append((model_input, output[0]))
        return output[0]

    # Take in ending info, use this for training purposes
    # Includes final player statuses, as well as all hidden roles
    '''
    {0: (role_int, status_int),
    ...
    num_players: (role_int, status_int)}
    '''

    def end(self, ending_info: dict):
        if self.doc:
            mafia_left = 2
            for i in ending_info:
                if ending_info[i][1] == 0:
                    if ending_info[i][0] == 1:
                        self.detective_kills += 1
                    if ending_info[i][0] == 2:
                        mafia_left -= 1
            if mafia_left > 0:
                self.mafia_wins += 1
            self.avg_last_day += self.last_day
        sarf = list()
        if ending_info[self.id][1] == 1:
            self.survived += 1
        print(self.survived / self.game)
        if len(self.state_action_pairs) < 4:
            pass
        for i in reversed(range(0, len(self.state_action_pairs)-1)):
            if i <= len(self.state_action_pairs) - 2:
                sarf.append((self.state_action_pairs[i][0],
                             self.reward.get_reward(
                            self.state_action_pairs[i][0],
                            ending_info,
                            self.state_action_pairs[i + 1][1],
                            self.num_players,
                            self.result,
                            i)))
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

    def __init__(self, predictor_model, train_model):
        self.predictor_model = predictor_model
        self.train_model = train_model
        self.in_use = Lock()
        self.accuracy = list()
        self.loss = list()
        self.val_accuracy = list()
        self.val_loss = list()

    def predict(self, input_data):
        self.in_use.acquire()
        try:
            output = self.predictor_model.predict(input_data)
        except ValueError as e:
            print(e)
        self.in_use.release()
        return output

    def train(self, info):
        info = np.array(info)
        input_data = list()
        output_data = list()
        # for i in random.sample(range(0, len(info[:, 0])), random.randint(0, len(info[:, 0]))):
        for i in range(0, len(info)):
            input_data.append(np.array(info[i][0]).reshape(-1, 1))
            output_data.append(info[i][1].reshape(-1, 1))

        if len(input_data) > 0:
            combined = list(zip(input_data, output_data))
            random.shuffle(combined)
            input_data[:], output_data[:] = zip(*combined)

            input_data = np.array(input_data)[:, :, 0]
            output_data = np.array(output_data)[:, :, 0]
            self.in_use.acquire()
            try:
                history = self.train_model.fit(input_data,
                               output_data, epochs=1, batch_size=4)
                self.loss.extend(history.history['loss'])
                self.val_loss.extend(history.history['val_loss'])
                self.accuracy.extend(history.history['acc'])
                self.val_accuracy.extend(history.history['val_acc'])
            except ValueError as e:
                print(e)
            except:
                pass
            self.in_use.release()
