from Client import Bot
from random import randint


class Reward():
    def get_reward(self, state, action) -> int:
        1


class DQNAgent(Bot):

    def __init__(self, times: int, reward: Reward, model):
        super().__init__(times)
        self.reward = reward
        self.model = model
        self.state_action_pairs = list()

    # Take in the action info, plug into NN to get action,
    # Save action input, and output for future reward calculations.
    '''
    Action info is as follows:
    {'Player': int,
     'Role': int,
     'Status': int,
     'Phase': int, 
     'Day': 0, 
     0: (status_int, list of length num_players of this players last guesses),
     ...
     num_players: (status_int, list of length num_players of this players last guesses),
     '''

    def action(self, action_info: dict) -> list:
        model_input = list()
        model_input.append(action_info['Player'])
        model_input.append(action_info['Role'])
        model_input.append(action_info['Status'])
        model_input.append(action_info['Phase'])
        model_input.append(action_info['Day'])
        for i in range(0, self.num_players):
            model_input.append(action_info[i][0])
            for j in range(0, self.num_players):
                model_input.append(action_info[i][1][j])
        output = self.model.predict(model_input)
        self.state_action_pairs.append(model_input, output)
        return output

    # Take in ending info, use this for training purposes
    # Includes final player statuses, as well as all hidden roles
    '''
    {0: (role_int, status_int),
    ...
    num_players: (role_int, status_int)}
    '''

    def end(self, ending_info: dict):
        print("poopoo")

    # Take in info_info, use to store responses from votes
    # Should almost always be a dead status, mainly used
    # to learn confirmed roles of the dead.
    '''
    (player_int, role_int, status_int)
    '''

    def info(self, info_info: tuple):
        print("peepee")
