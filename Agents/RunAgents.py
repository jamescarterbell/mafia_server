from DQN import DQNAgent, QReward, Trainer
from concurrent.futures import ThreadPoolExecutor
from tensorflow.keras.models import Sequential
from tensorflow.keras.layers import Dense, Activation, Input, Convolution1D, Flatten, LeakyReLU
from tensorflow.keras.models import Model
from tensorflow.keras import optimizers
from threading import Thread
import numpy as np
import tensorflow.keras.models as models
from tensorflow.keras.models import load_model

while(True):
    reward = QReward(.5)
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
        model.compile(loss='mean_squared_error', optimizer=optimizer)

    input_test = [i for i in range(0, 85)]

    model_train= models.clone_model(model)
    model_train.set_weights(model.get_weights())
    optimizer = optimizers.Adagrad(lr=0.01)
    model_train.compile(loss='mean_squared_error', optimizer=optimizer)

    for i in range(0, 1):
        print(model.fit(x=np.array(input_test).reshape(1, -1),
                        y=np.array([0 for i in range(0, 8)]).reshape(1, -1)))
        print(model_train.fit(x=np.array(input_test).reshape(1, -1),
                        y=np.array([0 for i in range(0, 8)]).reshape(1, -1)))

    trainer = Trainer(model, model_train)

    pool = ThreadPoolExecutor(max_workers=255)

    for i in range(0, 255):
        pool.submit(
            DQNAgent(5, reward, model=trainer, epsilon=.9).run_bot_join)

    doc_bot = DQNAgent(5, reward, model=trainer, epsilon=.9, document=True)
    doc_bot.run_bot_join()
    pool.shutdown(wait=True)

    with open("train_data.txt", "a") as f:
        f.write("{}, {}, {}\n".format(doc_bot.mafia_wins, doc_bot.detective_kills, doc_bot.avg_last_day))

    with open("accuracy.txt", "a") as f:
        for num in trainer.accuracy:
            f.write("{}\n".format(num))

    with open("loss.txt", "a") as f:
        for num in trainer.loss:
            f.write("{}\n".format(num))

    with open("val_accuracy.txt", "a") as f:
        for num in trainer.val_accuracy:
            f.write("{}\n".format(num))

    with open("val_loss.txt", "a") as f:
        for num in trainer.val_loss:
            f.write("{}\n".format(num))

    print("Shut down!")
    trainer.train_model.save('trained_model.h5')
