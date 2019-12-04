from Client import Bot, Reward
from random import randint
from threading import Lock
import numpy as np
import time
import queue
import random
import tensorflow as tf

global graph
graph = tf.get_default_graph()


class QReward(Reward):

    # Store's future reward modifier
    def __init__(self):
        pass

    # Calculate the reward from the given action,
    # and the max reward of the following state
    '''
    (state_info, action_taken)
    '''

    def get_reward(self, state: list, hidden_info, num_players: int, res=None, num=0) -> list():

        reward = list()
        can_win, hunt = self.check_win(
            state, num_players, hidden_info, state[1])

        # Calculate the reward value for each vote
        for i in range(0, num_players):
            reward_calc = 0
            if state[5 + i * (num_players + 2)] == 0:
                reward_calc -= .1
                reward.append(reward_calc)
                continue
            reward_calc += .1 if state[1] == 2 and hidden_info[i][0] < 2 else -0.1
            reward_calc += .2 if state[1] != 2 and hidden_info[i][0] == 2 else -0.1

            if can_win:
                if i in hunt:
                    reward_calc += .2

            reward.append(reward_calc)
        return np.array(reward).reshape(1, -1)

    def check_win(self, state: list, num_players, hidden_info, player_role):
        innocent_left = 0
        innocent_members = list()
        mafia_left = 0
        mafia_members = list()
        for i in range(0, num_players):
            if state[5 + i * (num_players + 2)] == 1:
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

    def __init__(self, times: int, reward: Reward, model, epsilon, gamma, document=False, train=True):
        super().__init__(times)
        self.reward = reward
        self.model = model
        self.game = 0.0
        self.survived = 0.0
        self.doc = document
        self.eps = epsilon
        self.gam = gamma
        self.games_played = 0
        self.train = train
        if self.doc:
            self.mafia_wins = 0
            self.detective_kills = 0
            self.avg_last_day = 0
            self.dead_vote = 0

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
        self.known = {}
        self.game += 1
        self.alive = True
        self.result = {}
        self.last_day = 0
        self.games_played = 0
        self.actions = list()

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
                model_input.extend(action_info[i][1])
            else:
                model_input.extend([0 for k in range(0, self.num_players)])

        into_model = np.array(model_input)

        if r < self.eps:
            try:
                output = self.model.predict(into_model)
            except:
                print("PREDICTION FAILED")
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

        if self.train:
            self.actions.append(model_input)

        if action_info[list(output[0]).index(max(output[0]))] == 0:
            self.dead_vote += 1

        return output[0]

    # Take in ending info, use this for training purposes
    # Includes final player statuses, as well as all hidden roles
    '''
    {0: (role_int, status_int),
    ...
    num_players: (role_int, status_int)}
    '''

    def end(self, ending_info: dict):
        if self.train:
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
            if ending_info[self.id][1] == 1:
                self.survived += 1

            self.model.remember(self.actions, ending_info)
            self.games_played += 1

    # Take in info_info, use to store responses from votes
    # Should almost always be a dead status, mainly used
    # to learn confirmed roles of the dead.
    '''
    (player_int, role_int, status_int)
    '''

    def info(self, info_info: tuple):
        self.result[len(self.actions) - 1] = info_info
        self.known[info_info[0]] = info_info[1]
        pass


class Trainer():

    def __init__(self, predictor_model, train_model, gamma, reward, bots, games):
        self.predictor_model = predictor_model
        self.train_model = train_model
        self.p_in_use = Lock()
        self.t_in_use = Lock()
        self.accuracy = list()
        self.loss = list()
        self.val_accuracy = list()
        self.val_loss = list()
        self.sarn_hid = queue.Queue()
        self.done_count = queue.Queue()
        self.gam = gamma
        self.reward = reward
        self.recalling = False
        self.done = False
        self.bots = bots
        self.games = games

    def predict(self, input_data):
        input_data = np.reshape(input_data, [-1, 85])
        self.p_in_use.acquire()
        with graph.as_default():
            try:
                output = self.predictor_model.predict(input_data)
            except ValueError as e:
                print("Predition Error\n")
                print(e)
            self.p_in_use.release()
        return output

    def remember(self, inputs, hidden):
        random.shuffle(inputs)
        for i in range(0, len(inputs) - 1):
            self.sarn_hid.put((inputs[i], hidden, inputs[i + 1]))
        self.done_count.put(1)

    def send_done(self):
        self.done_count.put(1)

    def recall(self):
        done_check = 0
        with graph.as_default():
            while done_check < ((self.games-1) * self.bots):
                if not self.done_count.empty():
                    done_check += self.done_count.get()
                while not self.sarn_hid.empty():
                    sample = self.sarn_hid.get()
                    reward = self.reward.get_reward(
                        sample[0], sample[1], 8)[0]
                    reward = np.add(reward, np.full(reward.shape, np.amax(self.t_predict(
                        sample[2])[0]) * self.gam))
                    self.train(sample[0], reward)
        print("FINISHED!")

    def train(self, state, reward):
        state = np.reshape(np.array(state), [-1, 85])
        reward = np.reshape(np.array(reward), [-1, 8])
        self.p_in_use.acquire()
        try:
            history = self.predictor_model.fit(state,
                                               reward, epochs=1, batch_size=1)
            self.loss.extend(history.history['loss'])
            self.val_loss.extend(history.history['val_loss'])
            self.accuracy.extend(history.history['acc'])
            self.val_accuracy.extend(history.history['val_acc'])
        except ValueError as e:
            print("Train Error\n")
            print(e)
        except:
            pass
        self.p_in_use.release()

    def t_predict(self, input_data):
        input_data = np.reshape(input_data, [-1, 85])
        try:
            output = self.train_model.predict(input_data)
        except ValueError as e:
            print("Predition Error\n")
            print(e)
        return output
