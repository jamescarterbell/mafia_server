from DQN import DQNAgent, Reward
from multiprocessing import Pool
from keras.models import Sequential
from keras.layers import Dense, Activation

reward = Reward()
model = Sequential([
    Dense(32, input_shape=(77,)),
    Activation('sigmoid'),
    Dense(16),
    Activation('sigmoid'),
    Dense(16),
    Activation('sigmoid'),
    Dense(8),
    Activation('softmax')
])
pool = Pool(8)
bots = [DQNAgent(10, reward, model=model) for i in range(0, 8)]

for bot in bots:
    pool.apply_async(bot.run_bot_join)
pool.close()
pool.join()
