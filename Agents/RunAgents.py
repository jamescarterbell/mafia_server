from DQN import DQNAgent, QReward, Trainer
from concurrent.futures import ThreadPoolExecutor
from tensorflow.keras.models import Sequential
from tensorflow.keras.layers import Dense, Activation, Input, Convolution1D, Flatten, LeakyReLU, MaxPooling1D
from tensorflow.keras.models import Model
from tensorflow.keras import optimizers
from threading import Thread
import numpy as np
import tensorflow.keras.models as models
from tensorflow.keras.models import load_model
import tensorflow as tf
import time

num_bots = 64
opts = tf.GPUOptions(per_process_gpu_memory_fraction=1)
config = tf.ConfigProto(gpu_options=opts)
config.gpu_options.allow_growth = True
sess = tf.Session(config=config)

while(True):
    reward = QReward()
    model = None
    try:
        model = load_model('trained_model.h5')
        print("Loading Model!")
    except:
        # Defining a model here to pass to the trainer object.
        # The trainer object keeps the model on the main thread
        # so that multiple agents can train on the same model
        # from multiple threads.
        inputs = Input(shape=(85, ))
        den_1 = Dense(64)(inputs)
        elu_1 = LeakyReLU(alpha=.3)(den_1)
        den_2 = Dense(64)(elu_1)
        elu_2 = LeakyReLU(alpha=.3)(den_2)
        den_3 = Dense(32)(elu_2)
        elu_3 = LeakyReLU(alpha=.3)(den_3)
        den_4 = Dense(32)(elu_3)
        elu_4 = LeakyReLU(alpha=.3)(den_4)
        den_5 = Dense(16)(elu_4)
        elu_5 = LeakyReLU(alpha=.3)(den_5)
        den_6 = Dense(16)(elu_5)
        elu_6 = LeakyReLU(alpha=.3)(den_6)
        den_7 = Dense(8)(elu_6)

        model = Model(inputs=inputs, outputs=den_7)
        optimizer = optimizers.Adagrad(lr=0.01)

    model_train = models.clone_model(model)
    model_train.set_weights(model.get_weights())
    optimizer = optimizers.Adagrad(lr=0.001)
    model.compile(loss='mean_squared_error', optimizer=optimizer)
    model_train.compile(loss='mean_squared_error', optimizer=optimizer)

    trainer = Trainer(model, model_train, .9, reward, num_bots, 5)

    pool = ThreadPoolExecutor(max_workers=num_bots)

    agents = list()

    for i in range(0, num_bots):
        agents.append(
            DQNAgent(5, reward, model=trainer, epsilon=.95, gamma=.75, document=True))
        pool.submit(
            agents[-1].run_bot_join)

    trainer.recall()
    pool.shutdown(wait=False)
    del(pool)

    max_day = 0
    avg_mafia_wins = 0
    avg_detective_kills = 0
    for agent in agents:
        max_day += agent.avg_last_day/5
        avg_detective_kills += agent.detective_kills
        avg_mafia_wins += agent.mafia_wins
    avg_detective_kills /= len(agents)
    avg_mafia_wins /= len(agents)
    max_day /= len(agents)

    with open("train_data.txt", "a") as f:
        f.write("{}, {}, {}\n".format(avg_mafia_wins,
                                      avg_detective_kills, max_day))

    with open("loss.txt", "a") as f:
        for num in trainer.loss:
            f.write("{}\n".format(num))

    print("Shut down!")
    trainer.predictor_model.save('trained_model.h5')
