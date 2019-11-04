from DQN import DQNAgent, QReward, Trainer
from concurrent.futures import ThreadPoolExecutor
from tensorflow.keras.models import Sequential
from tensorflow.keras.layers import Dense, Activation, Input
from tensorflow.keras.models import Model
from threading import Thread
import numpy as np

reward = QReward(.75)

# Defining a model here to pass to the trainer object.
# The trainer object keeps the model on the main thread
# so that multiple agents can train on the same model
# from multiple threads.
inputs = Input(shape=(85, ))
hid_1 = Dense(32, activation="sigmoid")(inputs)
hid_2 = Dense(32, activation="sigmoid")(hid_1)
hid_3 = Dense(16, activation="sigmoid")(hid_2)
hid_4 = Dense(16, activation="sigmoid")(hid_3)
predictions = Dense(8, activation="elu")(hid_4)

model = Model(inputs=inputs, outputs=predictions)
model.compile(loss='mean_squared_error', optimizer='sgd')

print(model.predict(np.array([i for i in range(0, 85)]).reshape(1, -1)))


trainer = Trainer(model)

pool = ThreadPoolExecutor(max_workers=7)

for i in range(0, 7):
    pool.submit(
        DQNAgent(1000, reward, model=trainer).run_bot_join)

DQNAgent(1000, reward, model=trainer).run_bot_join()
